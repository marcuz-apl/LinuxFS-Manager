use linuxfs_core::{
    BlockGeometry, BlockReader, Error, ErrorCategory, FilesystemInfo, Partition, PartitionReader,
    Result, SourceLayout, discover_layout, validate_read_range,
};
use std::{
    fs::File, os::windows::fs::FileExt, os::windows::io::AsRawHandle, path::PathBuf, sync::Arc,
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
    pub source_path: PathBuf,
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
    let mut partition_failures = Vec::new();
    for partition in partitions {
        let view = match PartitionReader::new(
            Arc::clone(&reader),
            partition.byte_offset,
            partition.byte_length,
        ) {
            Ok(view) => Arc::new(view) as Arc<dyn BlockReader>,
            Err(error) => {
                partition_failures.push(format!("partition {}: {error}", partition.number));
                continue;
            }
        };
        let backend = match linuxfs_ext::ExtReadOnlyBackend::open(view) {
            Ok(backend) => backend,
            Err(error) => {
                partition_failures.push(format!("partition {}: {error}", partition.number));
                continue;
            }
        };
        results.push(PhysicalPartitionInfo {
            disk_index,
            source_path: physical_disk_path(disk_index),
            partition,
            filesystem: backend.info()?,
        });
    }
    if results.is_empty() && !partition_failures.is_empty() {
        return Err(Error::new(
            ErrorCategory::UnsupportedFilesystem,
            format!(
                "no supported Ext partition; {}",
                partition_failures.join("; ")
            ),
        ));
    }
    Ok(results)
}

/// Probes drive-letter volume devices as a fallback for Windows configurations
/// where raw-disk partition enumeration does not expose Linux partitions.
pub fn discover_volume_partitions() -> Vec<PhysicalPartitionInfo> {
    (b'A'..=b'Z')
        .filter_map(|letter| {
            let path = PathBuf::from(format!(r"\\.\{}:", char::from(letter)));
            let file = open_physical_disk(&path).ok()?;
            let size_bytes = physical_disk_size(&file).ok()?;
            let reader = Arc::new(PhysicalDiskReader { file, size_bytes }) as Arc<dyn BlockReader>;
            let backend = linuxfs_ext::ExtReadOnlyBackend::open(reader).ok()?;
            Some(PhysicalPartitionInfo {
                disk_index: u32::MAX,
                source_path: path,
                partition: Partition {
                    number: u32::from(letter),
                    byte_offset: 0,
                    byte_length: size_bytes,
                    type_identifier: linuxfs_core::PartitionType::Mbr(0x83),
                    unique_identifier: None,
                },
                filesystem: backend.info().ok()?,
            })
        })
        .collect()
}

pub fn discover_physical_partitions(max_index: u32) -> Vec<PhysicalPartitionInfo> {
    discover_physical_partitions_checked(max_index).unwrap_or_default()
}

pub fn discover_physical_partitions_checked(max_index: u32) -> Result<Vec<PhysicalPartitionInfo>> {
    let mut opened = 0_u32;
    let mut results = Vec::new();
    let mut failures = Vec::new();
    (0..max_index).for_each(|index| {
        let Ok(reader) = PhysicalDiskReader::open(index) else {
            return;
        };
        opened = opened.saturating_add(1);
        let reader = Arc::new(reader) as Arc<dyn BlockReader>;
        match probe_physical_partitions(index, reader) {
            Ok(partitions) if partitions.is_empty() => {
                failures.push(format!("PhysicalDrive{index}: no supported Ext partition"));
            }
            Ok(mut partitions) => results.append(&mut partitions),
            Err(error) => failures.push(format!("PhysicalDrive{index}: {error}")),
        }
    });
    if opened == 0 {
        return Err(Error::new(
            ErrorCategory::StorageAccess,
            "no physical disks could be opened read-only; run as administrator if Windows denies raw-disk access",
        ));
    }
    if results.is_empty() {
        let detail = failures.join("; ");
        return Err(Error::new(
            ErrorCategory::UnsupportedFilesystem,
            format!("no supported Ext partition found ({detail})"),
        ));
    }
    Ok(results)
}

impl PhysicalDiskReader {
    pub fn open(index: u32) -> Result<Self> {
        let path = physical_disk_path(index);
        let file = open_physical_disk(&path)?;
        let size_bytes = physical_disk_size(&file)?;
        Ok(Self { file, size_bytes })
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn open_physical_disk(path: &std::path::Path) -> Result<File> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a valid NUL-terminated path and all flags request
    // read-only access with sharing enabled for other Windows disk users.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::with_source(
            ErrorCategory::StorageAccess,
            format!("cannot open {} read-only", path.display()),
            std::io::Error::last_os_error(),
        ));
    }
    // SAFETY: `handle` is a valid owned Windows file handle from CreateFileW.
    Ok(unsafe { File::from_raw_handle(handle as _) })
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

#[cfg(windows)]
#[allow(unsafe_code)]
fn physical_disk_size(file: &File) -> Result<u64> {
    use windows_sys::Win32::System::IO::DeviceIoControl;

    const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007_405c;

    let mut size = 0_i64;
    let mut returned = 0_u32;
    // SAFETY: the handle belongs to `file`; the input buffer is empty and the
    // output buffer is a valid writable DISK_LENGTH_INFO-compatible i64.
    let success = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            IOCTL_DISK_GET_LENGTH_INFO,
            std::ptr::null(),
            0,
            (&mut size as *mut i64).cast(),
            std::mem::size_of::<i64>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    if success == 0 || returned < std::mem::size_of::<i64>() as u32 || size < 0 {
        return Err(Error::new(
            ErrorCategory::StorageAccess,
            "cannot determine physical disk size",
        ));
    }
    u64::try_from(size).map_err(|_| {
        Error::new(
            ErrorCategory::StorageAccess,
            "physical disk size does not fit u64",
        )
    })
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
