use linuxfs_core::{
    BlockGeometry, BlockReader, Error, ErrorCategory, FilesystemInfo, Partition, PartitionReader,
    Result, SourceLayout, discover_layout, validate_read_range,
};
use std::{
    fs::{File, OpenOptions},
    os::windows::fs::FileExt,
    path::PathBuf,
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalDiskInfo {
    pub index: u32,
    pub source_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub struct PhysicalDiskReader {
    file: File,
    size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct PhysicalPartitionInfo {
    pub disk_index: u32,
    pub partition: Partition,
    pub filesystem: FilesystemInfo,
}

/// Probes a physical disk using only bounded reads and returns supported Ext partitions.
pub fn probe_physical_partitions(
    disk_index: u32,
    reader: Arc<dyn BlockReader>,
) -> Result<Vec<PhysicalPartitionInfo>> {
    let layout = discover_layout(reader.as_ref(), BlockGeometry::raw_image_512())?;
    let partitions = match layout {
        SourceLayout::Mbr { partitions } | SourceLayout::Gpt { partitions } => partitions,
        SourceLayout::DirectImage => return Ok(Vec::new()),
    };

    let mut results = Vec::new();
    for partition in partitions {
        let view = match PartitionReader::new(
            Arc::clone(&reader),
            partition.byte_offset,
            partition.byte_length,
        ) {
            Ok(view) => Arc::new(view) as Arc<dyn BlockReader>,
            Err(_) => continue,
        };
        let backend = match linuxfs_ext::ExtReadOnlyBackend::open(view) {
            Ok(backend) => backend,
            Err(_) => continue,
        };
        results.push(PhysicalPartitionInfo {
            disk_index,
            partition,
            filesystem: backend.info()?,
        });
    }
    Ok(results)
}

pub fn discover_physical_partitions(max_index: u32) -> Vec<PhysicalPartitionInfo> {
    (0..max_index)
        .filter_map(|index| {
            let reader = Arc::new(PhysicalDiskReader::open(index).ok()?) as Arc<dyn BlockReader>;
            probe_physical_partitions(index, reader).ok()
        })
        .flatten()
        .collect()
}

impl PhysicalDiskReader {
    pub fn open(index: u32) -> Result<Self> {
        let path = physical_disk_path(index);
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|source| {
                Error::with_source(
                    ErrorCategory::StorageAccess,
                    "cannot open physical disk read-only",
                    source,
                )
            })?;
        let size_bytes = file
            .metadata()
            .map_err(|source| {
                Error::with_source(
                    ErrorCategory::StorageAccess,
                    "cannot read physical disk metadata",
                    source,
                )
            })?
            .len();
        Ok(Self { file, size_bytes })
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

impl BlockReader for PhysicalDiskReader {
    fn len(&self) -> Result<u64> {
        Ok(self.size_bytes)
    }

    fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
        validate_read_range(self.size_bytes, offset, destination.len())?;
        let mut total = 0usize;
        while total < destination.len() {
            let read_offset = offset
                .checked_add(u64::try_from(total).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical read offset does not fit u64",
                    )
                })?)
                .ok_or_else(|| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical read offset overflow",
                    )
                })?;
            let read = self
                .file
                .seek_read(&mut destination[total..], read_offset)
                .map_err(|source| {
                    Error::with_source(
                        ErrorCategory::StorageAccess,
                        "cannot read physical disk",
                        source,
                    )
                })?;
            if read == 0 {
                return Err(Error::new(
                    ErrorCategory::StorageAccess,
                    "physical disk ended during read",
                ));
            }
            total += read;
        }
        Ok(())
    }
}

pub fn physical_disk_path(index: u32) -> PathBuf {
    PathBuf::from(format!(r"\\.\PhysicalDrive{index}"))
}

pub fn discover_physical_disks(max_index: u32) -> Vec<PhysicalDiskInfo> {
    (0..max_index)
        .filter_map(|index| {
            let reader = PhysicalDiskReader::open(index).ok()?;
            Some(PhysicalDiskInfo {
                index,
                source_path: physical_disk_path(index),
                size_bytes: reader.size_bytes(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryReader {
        bytes: Vec<u8>,
    }

    impl BlockReader for MemoryReader {
        fn len(&self) -> Result<u64> {
            Ok(self.bytes.len() as u64)
        }

        fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
            validate_read_range(self.bytes.len() as u64, offset, destination.len())?;
            let start = usize::try_from(offset).map_err(|_| {
                Error::new(ErrorCategory::StorageAccess, "offset does not fit usize")
            })?;
            destination.copy_from_slice(&self.bytes[start..start + destination.len()]);
            Ok(())
        }
    }

    #[test]
    fn physical_paths_are_bounded_by_numeric_index() {
        assert_eq!(
            physical_disk_path(0).to_string_lossy(),
            r"\\.\PhysicalDrive0"
        );
        assert_eq!(
            physical_disk_path(12).to_string_lossy(),
            r"\\.\PhysicalDrive12"
        );
    }

    #[test]
    fn discovery_with_zero_limit_is_empty() {
        assert!(discover_physical_disks(0).is_empty());
    }

    #[test]
    fn synthetic_direct_source_has_no_physical_partitions() {
        let reader: Arc<dyn BlockReader> = Arc::new(MemoryReader {
            bytes: vec![0; 4096],
        });
        let partitions = probe_physical_partitions(7, reader).expect("synthetic probe succeeds");
        assert!(partitions.is_empty());
    }
}
