use std::sync::Arc;

use linuxfs_core::{
    BlockReader, DirectoryEntry, Error, ErrorCategory, FileKind, FilesystemInfo, FsPath,
    NodeMetadata, ReadOnlyFilesystem, Result,
};
use xfs::{FileType, Inode, InodeFormat, Superblock};

const MAX_MATERIALIZED_IMAGE: u64 = 512 * 1024 * 1024;
const MAX_MATERIALIZED_FILE: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct XfsReadOnlyBackend {
    image: Arc<Vec<u8>>,
    superblock: Superblock,
}

impl XfsReadOnlyBackend {
    pub fn open(reader: Arc<dyn BlockReader>) -> Result<Self> {
        let length = reader.len()?;
        if length > MAX_MATERIALIZED_IMAGE {
            return Err(Error::new(
                ErrorCategory::UnsupportedFeature,
                "XFS source exceeds the safe 512 MiB reader limit; streaming XFS support is not available yet",
            ));
        }
        let size = usize::try_from(length).map_err(|_| {
            Error::new(
                ErrorCategory::UnsupportedFeature,
                "XFS source is too large for this platform",
            )
        })?;
        let mut image = vec![0u8; size];
        reader.read_exact_at(0, &mut image)?;
        let superblock = Superblock::parse(&image).map_err(map_error)?;
        Ok(Self {
            image: Arc::new(image),
            superblock,
        })
    }

    fn inode(&self, inode: u64) -> Result<Inode> {
        self.superblock
            .read_inode(&self.image, inode)
            .map_err(map_error)
    }

    fn resolve(&self, path: &FsPath) -> Result<Inode> {
        let mut current = self.inode(self.superblock.rootino)?;
        for component in path
            .as_str()
            .split('/')
            .filter(|component| !component.is_empty())
        {
            let entries = self
                .superblock
                .read_dir(&self.image, &current)
                .map_err(map_error)?;
            let entry = entries
                .iter()
                .find(|entry| entry.name == component.as_bytes())
                .ok_or_else(|| {
                    Error::new(
                        ErrorCategory::FilesystemCorrupt,
                        format!("XFS path component not found: {component}"),
                    )
                })?;
            current = self.inode(entry.inode)?;
        }
        Ok(current)
    }

    fn metadata_for(&self, inode: &Inode) -> NodeMetadata {
        NodeMetadata {
            kind: file_kind(inode.file_type()),
            size: inode.size,
            permissions: inode.mode & 0o7777,
            uid: 0,
            gid: 0,
        }
    }
}

impl ReadOnlyFilesystem for XfsReadOnlyBackend {
    fn info(&self) -> Result<FilesystemInfo> {
        let total_size = u64::from(self.superblock.agblocks)
            .checked_mul(u64::from(self.superblock.agcount))
            .and_then(|blocks| blocks.checked_mul(u64::from(self.superblock.blocksize)));
        Ok(FilesystemInfo {
            filesystem_type: "XFS".to_owned(),
            label: None,
            uuid: None,
            block_size: Some(self.superblock.blocksize),
            total_size,
            free_size: None,
        })
    }

    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata> {
        Ok(self.metadata_for(&self.resolve(path)?))
    }

    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>> {
        let inode = self.resolve(path)?;
        let entries = self
            .superblock
            .read_dir(&self.image, &inode)
            .map_err(map_error)?;
        entries
            .into_iter()
            .map(|entry| {
                let child = self.inode(entry.inode)?;
                let name = String::from_utf8(entry.name).map_err(|_| {
                    Error::new(
                        ErrorCategory::FilesystemCorrupt,
                        "XFS directory name is not valid UTF-8",
                    )
                })?;
                Ok(DirectoryEntry {
                    name,
                    metadata: self.metadata_for(&child),
                })
            })
            .collect()
    }

    fn read_file_at(&self, path: &FsPath, offset: u64, destination: &mut [u8]) -> Result<usize> {
        let inode = self.resolve(path)?;
        if !inode.is_reg() {
            return Err(Error::new(
                ErrorCategory::InvalidImage,
                "XFS path is not a regular file",
            ));
        }
        if inode.size > MAX_MATERIALIZED_FILE {
            return Err(Error::new(
                ErrorCategory::UnsupportedFeature,
                "XFS file exceeds the safe 64 MiB reader limit",
            ));
        }
        let file = self
            .superblock
            .read_file(&self.image, &inode)
            .map_err(map_error)?;
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let Some(slice) = file.get(start..) else {
            return Ok(0);
        };
        let count = slice.len().min(destination.len());
        destination[..count].copy_from_slice(&slice[..count]);
        Ok(count)
    }

    fn read_link(&self, path: &FsPath) -> Result<FsPath> {
        let inode = self.resolve(path)?;
        if inode.file_type() != FileType::Symlink {
            return Err(Error::new(
                ErrorCategory::InvalidImage,
                "XFS path is not a symbolic link",
            ));
        }
        let target = match inode.format {
            InodeFormat::Local => inode.data_fork[..usize::try_from(inode.size)
                .unwrap_or(0)
                .min(inode.data_fork.len())]
                .to_vec(),
            _ => self
                .superblock
                .read_file(&self.image, &inode)
                .map_err(map_error)?,
        };
        FsPath::parse(std::str::from_utf8(&target).map_err(|_| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                "XFS symlink target is not valid UTF-8",
            )
        })?)
    }
}

fn file_kind(kind: FileType) -> FileKind {
    match kind {
        FileType::Regular => FileKind::Regular,
        FileType::Directory => FileKind::Directory,
        FileType::Symlink => FileKind::Symlink,
        _ => FileKind::Other,
    }
}

fn map_error(error: xfs::XfsError) -> Error {
    Error::new(
        ErrorCategory::FilesystemCorrupt,
        format!("XFS operation failed: {error}"),
    )
}
