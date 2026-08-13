use std::sync::Arc;

use linuxfs_core::{
    BlockReader, DirectoryEntry, Error, ErrorCategory, FilesystemInfo, FsPath, NodeMetadata,
    ReadOnlyFilesystem, Result,
};

#[derive(Clone)]
pub enum ReadOnlyBackend {
    Ext(linuxfs_ext::ExtReadOnlyBackend),
    Squashfs(linuxfs_squashfs::SquashfsReadOnlyBackend),
    Xfs(linuxfs_xfs::XfsReadOnlyBackend),
}

impl ReadOnlyBackend {
    pub fn open(reader: Arc<dyn BlockReader>) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact_at(0, &mut magic)?;
        let backend = if magic == *b"hsqs" {
            Self::Squashfs(linuxfs_squashfs::SquashfsReadOnlyBackend::open(reader)?)
        } else if magic == *b"XFSB" {
            Self::Xfs(linuxfs_xfs::XfsReadOnlyBackend::open(reader)?)
        } else {
            Self::Ext(linuxfs_ext::ExtReadOnlyBackend::open(reader)?)
        };
        Ok(backend)
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Ext(_) => "ext2/ext3/ext4",
            Self::Squashfs(_) => "SquashFS 4.0",
            Self::Xfs(_) => "XFS",
        }
    }
}

impl ReadOnlyFilesystem for ReadOnlyBackend {
    fn info(&self) -> Result<FilesystemInfo> {
        match self {
            Self::Ext(v) => v.info(),
            Self::Squashfs(v) => v.info(),
            Self::Xfs(v) => v.info(),
        }
    }
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata> {
        match self {
            Self::Ext(v) => v.lookup(path),
            Self::Squashfs(v) => v.lookup(path),
            Self::Xfs(v) => v.lookup(path),
        }
    }
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>> {
        match self {
            Self::Ext(v) => v.read_dir(path),
            Self::Squashfs(v) => v.read_dir(path),
            Self::Xfs(v) => v.read_dir(path),
        }
    }
    fn read_file_at(&self, path: &FsPath, offset: u64, destination: &mut [u8]) -> Result<usize> {
        match self {
            Self::Ext(v) => v.read_file_at(path, offset, destination),
            Self::Squashfs(v) => v.read_file_at(path, offset, destination),
            Self::Xfs(v) => v.read_file_at(path, offset, destination),
        }
    }
    fn read_link(&self, path: &FsPath) -> Result<FsPath> {
        match self {
            Self::Ext(v) => v.read_link(path),
            Self::Squashfs(v) => v.read_link(path),
            Self::Xfs(v) => v.read_link(path),
        }
    }
}

pub fn open_read_only(reader: Arc<dyn BlockReader>) -> Result<ReadOnlyBackend> {
    ReadOnlyBackend::open(reader)
}

pub fn unsupported_summary() -> Error {
    Error::new(
        ErrorCategory::UnsupportedFilesystem,
        "no supported Ext, SquashFS, or XFS filesystem found",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryReader {
        bytes: Vec<u8>,
        reported_len: Option<u64>,
    }

    impl BlockReader for MemoryReader {
        fn len(&self) -> Result<u64> {
            Ok(self.reported_len.unwrap_or(self.bytes.len() as u64))
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
    fn squashfs_signature_is_routed_to_squashfs_backend() {
        let reader: Arc<dyn BlockReader> = Arc::new(MemoryReader {
            bytes: b"hsqs".to_vec(),
            reported_len: None,
        });
        let error = ReadOnlyBackend::open(reader)
            .err()
            .expect("truncated SquashFS must fail");
        assert_eq!(error.category(), ErrorCategory::FilesystemCorrupt);
    }

    #[test]
    fn oversized_xfs_is_rejected_before_materialization() {
        let reader: Arc<dyn BlockReader> = Arc::new(MemoryReader {
            bytes: b"XFSB".to_vec(),
            reported_len: Some(2 * 1024 * 1024 * 1024 + 1),
        });
        let error = ReadOnlyBackend::open(reader)
            .err()
            .expect("oversized XFS must fail closed");
        assert_eq!(error.category(), ErrorCategory::UnsupportedFeature);
    }
}
