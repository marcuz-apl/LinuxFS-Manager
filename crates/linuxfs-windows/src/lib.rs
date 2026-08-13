use linuxfs_core::{
    BlockGeometry, BlockReader, Error, ErrorCategory, FilesystemInfo, Partition, PartitionReader,
    ReadOnlyFilesystem, Result, SourceLayout, discover_layout, validate_read_range,
};
use std::{
    collections::VecDeque,
    fs::File,
    os::windows::fs::FileExt,
    os::windows::io::AsRawHandle,
    path::PathBuf,
    sync::{Arc, Mutex},
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
    cache: Mutex<PhysicalReadCache>,
}

const PHYSICAL_READ_CHUNK_SIZE: usize = 64 * 1024;
const PHYSICAL_READ_CACHE_CHUNKS: usize = 64;

#[derive(Debug, Default)]
struct PhysicalReadCache {
    entries: VecDeque<CachedPhysicalChunk>,
}

#[derive(Debug)]
struct CachedPhysicalChunk {
    offset: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PhysicalPartitionInfo {
    pub disk_index: u32,
    pub source_path: PathBuf,
    pub partition: Partition,
    pub filesystem: FilesystemInfo,
}

/// Results and bounded diagnostics from one physical-drive scan.
#[derive(Debug, Default)]
pub struct PhysicalScanReport {
    pub partitions: Vec<PhysicalPartitionInfo>,
    pub diagnostics: Vec<String>,
}

impl PhysicalScanReport {
    pub fn render(&self) -> String {
        let mut rendered = String::from("LinuxFS Manager physical scan diagnostics\n");
        rendered.push_str(&format!(
            "Discovered supported filesystem partitions: {}\n",
            self.partitions.len()
        ));
        for diagnostic in &self.diagnostics {
            rendered.push_str("- ");
            rendered.push_str(diagnostic);
            rendered.push('\n');
        }
        rendered
    }
}

/// Probes a physical disk using only bounded reads and returns supported filesystem partitions.
pub fn probe_physical_partitions(
    disk_index: u32,
    reader: Arc<dyn BlockReader>,
) -> Result<Vec<PhysicalPartitionInfo>> {
    probe_physical_partitions_with_diagnostics(disk_index, reader, &mut Vec::new())
}

fn probe_physical_partitions_with_diagnostics(
    disk_index: u32,
    reader: Arc<dyn BlockReader>,
    diagnostics: &mut Vec<String>,
) -> Result<Vec<PhysicalPartitionInfo>> {
    record_read_preview(reader.as_ref(), 0, "LBA 0", diagnostics);
    record_read_preview(reader.as_ref(), 512, "LBA 1", diagnostics);
    let layout = discover_layout(reader.as_ref(), BlockGeometry::raw_image_512())?;
    let partitions = match layout {
        SourceLayout::Mbr { partitions } => {
            diagnostics.push(format!("PhysicalDrive{disk_index}: layout=MBR"));
            partitions
        }
        SourceLayout::Gpt { partitions } => {
            diagnostics.push(format!(
                "PhysicalDrive{disk_index}: layout=GPT, partitions={}",
                partitions.len()
            ));
            partitions
        }
        SourceLayout::DirectImage => {
            diagnostics.push(format!(
                "PhysicalDrive{disk_index}: layout=direct image; no partition table"
            ));
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    let mut partition_failures = Vec::new();
    for partition in partitions {
        diagnostics.push(format!(
            "PhysicalDrive{disk_index}: partition {} offset={} length={} type={:?}",
            partition.number,
            partition.byte_offset,
            partition.byte_length,
            partition.type_identifier
        ));
        record_read_preview(
            reader.as_ref(),
            partition.byte_offset,
            &format!("partition {} start", partition.number),
            diagnostics,
        );
        record_ext_magic(
            reader.as_ref(),
            partition.byte_offset,
            partition.number,
            diagnostics,
        );
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
        let backend = match linuxfs_backends::ReadOnlyBackend::open(view) {
            Ok(backend) => backend,
            Err(error) => {
                diagnostics.push(format!(
                    "PhysicalDrive{disk_index}: partition {} filesystem probe failed: {error}",
                    partition.number
                ));
                partition_failures.push(format!("partition {}: {error}", partition.number));
                continue;
            }
        };
        diagnostics.push(format!(
            "PhysicalDrive{disk_index}: partition {} filesystem probe succeeded ({})",
            partition.number,
            backend.kind()
        ));
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
                "no supported filesystem partition; {}",
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
            let reader = Arc::new(PhysicalDiskReader {
                file,
                size_bytes,
                cache: Mutex::new(PhysicalReadCache::default()),
            }) as Arc<dyn BlockReader>;
            let backend = linuxfs_backends::ReadOnlyBackend::open(reader).ok()?;
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
    let report = scan_physical_partitions(max_index);
    if !report.partitions.is_empty() {
        return Ok(report.partitions);
    }
    if report.diagnostics.is_empty() {
        return Err(Error::new(
            ErrorCategory::StorageAccess,
            "physical scan produced no diagnostics",
        ));
    }
    Err(Error::new(
        ErrorCategory::UnsupportedFilesystem,
        report.render(),
    ))
}

pub fn scan_physical_partitions(max_index: u32) -> PhysicalScanReport {
    let mut report = PhysicalScanReport::default();
    for index in 0..max_index {
        let reader = match PhysicalDiskReader::open(index) {
            Ok(reader) => {
                report.diagnostics.push(format!(
                    "PhysicalDrive{index}: opened read-only, size={} bytes",
                    reader.size_bytes()
                ));
                reader
            }
            Err(error) => {
                report
                    .diagnostics
                    .push(format!("PhysicalDrive{index}: open failed: {error}"));
                continue;
            }
        };
        let reader = Arc::new(reader) as Arc<dyn BlockReader>;
        match probe_physical_partitions_with_diagnostics(index, reader, &mut report.diagnostics) {
            Ok(partitions) if partitions.is_empty() => report.diagnostics.push(format!(
                "PhysicalDrive{index}: no supported filesystem partition"
            )),
            Ok(mut partitions) => report.partitions.append(&mut partitions),
            Err(error) => report
                .diagnostics
                .push(format!("PhysicalDrive{index}: probe failed: {error}")),
        }
    }
    report
}

impl PhysicalDiskReader {
    pub fn open(index: u32) -> Result<Self> {
        let path = physical_disk_path(index);
        let file = open_physical_disk(&path)?;
        let size_bytes = physical_disk_size(&file)?;
        Ok(Self {
            file,
            size_bytes,
            cache: Mutex::new(PhysicalReadCache::default()),
        })
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
        if destination.is_empty() {
            return Ok(());
        }

        // PhysicalDrive devices may expose 4K physical sectors while still
        // accepting 512-byte logical-sector reads. Read aligned 64-KB chunks
        // instead of issuing one device request per 512-byte sector. The
        // bounded cache also avoids re-reading Ext metadata for each Explorer
        // callback while keeping memory use predictable.
        let end = offset
            .checked_add(u64::try_from(destination.len()).map_err(|_| {
                Error::new(
                    ErrorCategory::StorageAccess,
                    "physical read length does not fit u64",
                )
            })?)
            .ok_or_else(|| {
                Error::new(ErrorCategory::StorageAccess, "physical read range overflow")
            })?;
        let chunk_size = u64::try_from(PHYSICAL_READ_CHUNK_SIZE).map_err(|_| {
            Error::new(
                ErrorCategory::StorageAccess,
                "physical chunk size does not fit u64",
            )
        })?;
        let mut chunk_offset = offset / chunk_size * chunk_size;
        while chunk_offset < end {
            let available = self.size_bytes.checked_sub(chunk_offset).ok_or_else(|| {
                Error::new(
                    ErrorCategory::StorageAccess,
                    "physical chunk offset exceeds disk size",
                )
            })?;
            let chunk_len = usize::try_from(available.min(chunk_size)).map_err(|_| {
                Error::new(
                    ErrorCategory::StorageAccess,
                    "physical chunk length does not fit usize",
                )
            })?;
            let mut cache = self.cache.lock().map_err(|_| {
                Error::new(
                    ErrorCategory::StorageAccess,
                    "physical read cache lock is poisoned",
                )
            })?;
            let cache_index = cache
                .entries
                .iter()
                .position(|entry| entry.offset == chunk_offset);
            if let Some(index) = cache_index {
                let entry = cache.entries.remove(index).ok_or_else(|| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical read cache entry disappeared",
                    )
                })?;
                let copy_start = offset.max(chunk_offset);
                let copy_end = end.min(chunk_offset + entry.bytes.len() as u64);
                let source_start = usize::try_from(copy_start - chunk_offset).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical cache copy offset overflow",
                    )
                })?;
                let destination_start = usize::try_from(copy_start - offset).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical destination offset overflow",
                    )
                })?;
                let copy_len = usize::try_from(copy_end - copy_start).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical cache copy length overflow",
                    )
                })?;
                destination[destination_start..destination_start + copy_len]
                    .copy_from_slice(&entry.bytes[source_start..source_start + copy_len]);
                cache.entries.push_back(entry);
            } else {
                let mut bytes = vec![0_u8; chunk_len];
                read_physical_range(&self.file, chunk_offset, &mut bytes)?;
                let copy_start = offset.max(chunk_offset);
                let copy_end = end.min(chunk_offset + bytes.len() as u64);
                let source_start = usize::try_from(copy_start - chunk_offset).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical read copy offset overflow",
                    )
                })?;
                let destination_start = usize::try_from(copy_start - offset).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical destination offset overflow",
                    )
                })?;
                let copy_len = usize::try_from(copy_end - copy_start).map_err(|_| {
                    Error::new(
                        ErrorCategory::StorageAccess,
                        "physical read copy length overflow",
                    )
                })?;
                destination[destination_start..destination_start + copy_len]
                    .copy_from_slice(&bytes[source_start..source_start + copy_len]);
                cache.entries.push_back(CachedPhysicalChunk {
                    offset: chunk_offset,
                    bytes,
                });
                if cache.entries.len() > PHYSICAL_READ_CACHE_CHUNKS {
                    cache.entries.pop_front();
                }
            }
            drop(cache);
            chunk_offset = chunk_offset.checked_add(chunk_size).ok_or_else(|| {
                Error::new(
                    ErrorCategory::StorageAccess,
                    "physical chunk offset overflow",
                )
            })?;
        }
        Ok(())
    }
}

fn read_physical_range(file: &File, offset: u64, destination: &mut [u8]) -> Result<()> {
    let mut read_total = 0usize;
    while read_total < destination.len() {
        let read_offset = offset
            .checked_add(u64::try_from(read_total).map_err(|_| {
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
        let read = file
            .seek_read(&mut destination[read_total..], read_offset)
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
        read_total += read;
    }
    Ok(())
}

pub fn physical_disk_path(index: u32) -> PathBuf {
    PathBuf::from(format!(r"\\.\PhysicalDrive{index}"))
}

fn record_read_preview(
    reader: &dyn BlockReader,
    offset: u64,
    label: &str,
    diagnostics: &mut Vec<String>,
) {
    let mut bytes = [0_u8; 16];
    match reader.read_exact_at(offset, &mut bytes) {
        Ok(()) => diagnostics.push(format!("{label}: {}", hex_bytes(&bytes))),
        Err(error) => diagnostics.push(format!("{label}: read failed: {error}")),
    }
}

fn record_ext_magic(
    reader: &dyn BlockReader,
    partition_offset: u64,
    partition_number: u32,
    diagnostics: &mut Vec<String>,
) {
    let Some(superblock_offset) = partition_offset.checked_add(1024) else {
        diagnostics.push(format!(
            "partition {partition_number}: superblock offset overflow"
        ));
        return;
    };
    let Some(magic_offset) = superblock_offset.checked_add(56) else {
        diagnostics.push(format!(
            "partition {partition_number}: Ext magic offset overflow"
        ));
        return;
    };
    let mut magic = [0_u8; 2];
    match reader.read_exact_at(magic_offset, &mut magic) {
        Ok(()) => diagnostics.push(format!(
            "partition {partition_number}: Ext magic at +1080={}",
            hex_bytes(&magic)
        )),
        Err(error) => diagnostics.push(format!(
            "partition {partition_number}: Ext magic read failed: {error}"
        )),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
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

    #[test]
    fn physical_scan_report_renders_diagnostics() {
        let report = PhysicalScanReport {
            partitions: Vec::new(),
            diagnostics: vec!["PhysicalDrive3: open failed: Access is denied".to_owned()],
        };
        let rendered = report.render();
        assert!(rendered.contains("LinuxFS Manager physical scan diagnostics"));
        assert!(rendered.contains("PhysicalDrive3: open failed: Access is denied"));
        assert!(rendered.contains("Discovered supported filesystem partitions: 0"));
    }
}
