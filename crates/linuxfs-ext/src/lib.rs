use std::{error::Error as StdError, sync::Arc};

use ext4_view::{Ext4, Ext4Error, Ext4Read};
use linuxfs_core::{
    BlockReader, DirectoryEntry, Error, ErrorCategory, FileKind, FilesystemInfo, FsPath,
    NodeMetadata, ReadOnlyFilesystem, Result,
};

struct ReaderAdapter {
    reader: Arc<dyn BlockReader>,
}

#[derive(Debug)]
struct ReaderError(String);
impl std::fmt::Display for ReaderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl StdError for ReaderError {}

impl Ext4Read for ReaderAdapter {
    fn read(
        &mut self,
        start_byte: u64,
        destination: &mut [u8],
    ) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        self.reader
            .read_exact_at(start_byte, destination)
            .map_err(|error| {
                Box::new(ReaderError(error.to_string())) as Box<dyn StdError + Send + Sync>
            })
    }
}

#[derive(Clone)]
pub struct ExtReadOnlyBackend {
    reader: Arc<dyn BlockReader>,
}

impl ExtReadOnlyBackend {
    pub fn open(reader: Arc<dyn BlockReader>) -> Result<Self> {
        Self::load(&reader)?;
        Ok(Self { reader })
    }

    pub fn info(&self) -> Result<FilesystemInfo> {
        let filesystem = Self::load(&self.reader)?;
        let label = filesystem.label().to_str().ok().map(str::to_owned);
        let uuid = Some(*filesystem.uuid().as_bytes());
        let (block_size, total_size, free_size) = self.superblock_sizes()?;
        Ok(FilesystemInfo {
            filesystem_type: "ext2/ext3/ext4".to_owned(),
            label,
            uuid,
            block_size,
            total_size,
            free_size,
        })
    }

    fn superblock_sizes(&self) -> Result<(Option<u32>, Option<u64>, Option<u64>)> {
        let mut superblock = [0u8; 1024];
        self.reader.read_exact_at(1024, &mut superblock)?;
        let log_block_size = read_u32_le(&superblock, 0x18);
        let Some(block_size) = 1024u64.checked_shl(log_block_size) else {
            return Ok((None, None, None));
        };
        let Some(block_size) = u32::try_from(block_size).ok() else {
            return Ok((None, None, None));
        };
        let blocks = u64::from(read_u32_le(&superblock, 0x04))
            | (u64::from(read_u32_le(&superblock, 0x150)) << 32);
        let free_blocks = u64::from(read_u32_le(&superblock, 0x0c))
            | (u64::from(read_u32_le(&superblock, 0x158)) << 32);
        let total_size = blocks.checked_mul(u64::from(block_size));
        let free_size = free_blocks.checked_mul(u64::from(block_size));
        Ok((Some(block_size), total_size, free_size))
    }

    fn load(reader: &Arc<dyn BlockReader>) -> Result<Ext4> {
        Ext4::load(Box::new(ReaderAdapter {
            reader: Arc::clone(reader),
        }))
        .map_err(map_error)
    }

    fn metadata(&self, path: &FsPath) -> Result<NodeMetadata> {
        let filesystem = Self::load(&self.reader)?;
        let metadata = filesystem.metadata(path.as_str()).map_err(map_error)?;
        Ok(NodeMetadata {
            kind: file_kind(metadata.file_type()),
            size: metadata.len(),
            permissions: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        })
    }
}

impl ReadOnlyFilesystem for ExtReadOnlyBackend {
    fn info(&self) -> Result<FilesystemInfo> {
        self.info()
    }
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata> {
        self.metadata(path)
    }

    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>> {
        let filesystem = Self::load(&self.reader)?;
        filesystem
            .read_dir(path.as_str())
            .map_err(map_error)?
            .map(|entry| {
                let entry = entry.map_err(map_error)?;
                let metadata = entry.metadata().map_err(map_error)?;
                Ok(DirectoryEntry {
                    name: entry
                        .file_name()
                        .as_str()
                        .map_err(|_| {
                            Error::new(
                                ErrorCategory::FilesystemCorrupt,
                                "Ext directory name is not valid UTF-8",
                            )
                        })?
                        .to_owned(),
                    metadata: NodeMetadata {
                        kind: file_kind(metadata.file_type()),
                        size: metadata.len(),
                        permissions: metadata.mode(),
                        uid: metadata.uid(),
                        gid: metadata.gid(),
                    },
                })
            })
            .collect()
    }

    fn read_file_at(&self, path: &FsPath, offset: u64, destination: &mut [u8]) -> Result<usize> {
        let filesystem = Self::load(&self.reader)?;
        let mut file = filesystem.open(path.as_str()).map_err(map_error)?;
        file.seek_to(offset).map_err(map_error)?;
        let mut total = 0;
        while total < destination.len() {
            let read = file
                .read_bytes(&mut destination[total..])
                .map_err(map_error)?;
            if read == 0 {
                break;
            }
            total += read;
        }
        Ok(total)
    }

    fn read_link(&self, path: &FsPath) -> Result<FsPath> {
        let filesystem = Self::load(&self.reader)?;
        let target = filesystem.read_link(path.as_str()).map_err(map_error)?;
        FsPath::parse(target.to_str().map_err(|_| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                "Ext symlink target is not valid UTF-8",
            )
        })?)
    }
}

fn file_kind(file_type: ext4_view::FileType) -> FileKind {
    if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_regular_file() {
        FileKind::Regular
    } else {
        FileKind::Other
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn map_error(error: Ext4Error) -> Error {
    let category = match &error {
        Ext4Error::Io(_) => ErrorCategory::StorageAccess,
        Ext4Error::Incompatible(_) => ErrorCategory::UnsupportedFeature,
        Ext4Error::Encrypted => ErrorCategory::UnsupportedFilesystem,
        Ext4Error::Corrupt(_) => ErrorCategory::FilesystemCorrupt,
        _ => ErrorCategory::FilesystemCorrupt,
    };
    Error::new(
        category,
        format!("Ext filesystem operation failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use linuxfs_core::BlockReader;
    struct MemoryReader {
        bytes: Vec<u8>,
    }
    impl BlockReader for MemoryReader {
        fn len(&self) -> Result<u64> {
            Ok(self.bytes.len() as u64)
        }
        fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
            linuxfs_core::validate_read_range(self.bytes.len() as u64, offset, destination.len())?;
            let start = usize::try_from(offset)
                .map_err(|_| Error::new(ErrorCategory::StorageAccess, "offset overflow"))?;
            destination.copy_from_slice(&self.bytes[start..start + destination.len()]);
            Ok(())
        }
    }
    #[test]
    fn rejects_truncated_source_as_structured_error() {
        let reader: Arc<dyn BlockReader> = Arc::new(MemoryReader {
            bytes: vec![0; 1024],
        });
        let result = ExtReadOnlyBackend::open(reader);
        assert!(result.is_err());
        assert!(matches!(
            result.err().map(|error| error.category()),
            Some(ErrorCategory::StorageAccess | ErrorCategory::FilesystemCorrupt)
        ));
    }
}
