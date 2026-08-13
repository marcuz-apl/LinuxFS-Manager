use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::Arc,
};

use linuxfs_core::{
    BlockReader, DirectoryEntry, Error, ErrorCategory, FileKind, FilesystemInfo, FsPath,
    NodeMetadata, ReadOnlyFilesystem, Result,
};
use squashfs_reader::FileSystem;

struct ReaderCursor {
    reader: Arc<dyn BlockReader>,
    position: u64,
}

impl Read for ReaderCursor {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        let length = destination.len().min(
            usize::try_from(
                self.reader
                    .len()
                    .map_err(io_error)?
                    .saturating_sub(self.position),
            )
            .unwrap_or(0),
        );
        if length == 0 {
            return Ok(0);
        }
        self.reader
            .read_exact_at(self.position, &mut destination[..length])
            .map_err(io_error)?;
        self.position = self.position.saturating_add(length as u64);
        Ok(length)
    }
}

impl Seek for ReaderCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let length = self.reader.len().map_err(io_error)?;
        let next = match position {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(value) => i128::from(self.position) + i128::from(value),
            SeekFrom::End(value) => i128::from(length) + i128::from(value),
        };
        if next < 0 || next > i128::from(length) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek outside source",
            ));
        }
        self.position = u64::try_from(next)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))?;
        Ok(self.position)
    }
}

#[derive(Clone)]
pub struct SquashfsReadOnlyBackend {
    reader: Arc<dyn BlockReader>,
}

impl SquashfsReadOnlyBackend {
    pub fn open(reader: Arc<dyn BlockReader>) -> Result<Self> {
        FileSystem::from_read(ReaderCursor {
            reader: Arc::clone(&reader),
            position: 0,
        })
        .map_err(map_error)?;
        Ok(Self { reader })
    }
}

impl ReadOnlyFilesystem for SquashfsReadOnlyBackend {
    fn info(&self) -> Result<FilesystemInfo> {
        let mut header = [0u8; 96];
        self.reader.read_exact_at(0, &mut header)?;
        let block_size = u32::from_le_bytes(header[12..16].try_into().map_err(|_| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                "invalid SquashFS superblock",
            )
        })?);
        let bytes_used = u64::from_le_bytes(header[40..48].try_into().map_err(|_| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                "invalid SquashFS superblock",
            )
        })?);
        Ok(FilesystemInfo {
            filesystem_type: "SquashFS 4.0".to_owned(),
            label: None,
            uuid: None,
            block_size: Some(block_size),
            total_size: Some(bytes_used),
            free_size: Some(0),
        })
    }

    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata> {
        let fs = FileSystem::from_read(ReaderCursor {
            reader: Arc::clone(&self.reader),
            position: 0,
        })
        .map_err(map_error)?;
        let metadata = fs.metadata(path.as_str()).map_err(map_error)?;
        Ok(NodeMetadata {
            kind: file_kind(metadata.file_type()),
            size: metadata.len(),
            permissions: metadata.permissions(),
            uid: metadata.uid(&fs).map_err(map_error)?,
            gid: metadata.gid(&fs).map_err(map_error)?,
        })
    }

    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>> {
        let fs = FileSystem::from_read(ReaderCursor {
            reader: Arc::clone(&self.reader),
            position: 0,
        })
        .map_err(map_error)?;
        fs.read_dir(path.as_str())
            .map_err(map_error)?
            .map(|entry| {
                let entry = entry.map_err(map_error)?;
                let metadata = entry.metadata(&fs).map_err(map_error)?;
                Ok(DirectoryEntry {
                    name: entry.name().to_owned(),
                    metadata: NodeMetadata {
                        kind: file_kind(metadata.file_type()),
                        size: metadata.len(),
                        permissions: metadata.permissions(),
                        uid: metadata.uid(&fs).map_err(map_error)?,
                        gid: metadata.gid(&fs).map_err(map_error)?,
                    },
                })
            })
            .collect()
    }

    fn read_file_at(&self, path: &FsPath, offset: u64, destination: &mut [u8]) -> Result<usize> {
        let fs = FileSystem::from_read(ReaderCursor {
            reader: Arc::clone(&self.reader),
            position: 0,
        })
        .map_err(map_error)?;
        let mut file = fs.open(path.as_str()).map_err(map_error)?;
        file.seek(SeekFrom::Start(offset)).map_err(map_error)?;
        file.read(destination).map_err(map_error)
    }

    fn read_link(&self, path: &FsPath) -> Result<FsPath> {
        let fs = FileSystem::from_read(ReaderCursor {
            reader: Arc::clone(&self.reader),
            position: 0,
        })
        .map_err(map_error)?;
        let metadata = fs.metadata(path.as_str()).map_err(map_error)?;
        let target = metadata.target().ok_or_else(|| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                "SquashFS entry is not a symlink",
            )
        })?;
        FsPath::parse(target)
    }
}

fn file_kind(kind: squashfs_reader::FileType) -> FileKind {
    match kind {
        squashfs_reader::FileType::Directory => FileKind::Directory,
        squashfs_reader::FileType::Symlink => FileKind::Symlink,
        squashfs_reader::FileType::File => FileKind::Regular,
    }
}

fn io_error(error: Error) -> io::Error {
    io::Error::other(error.to_string())
}
fn map_error(error: io::Error) -> Error {
    let category = match error.kind() {
        io::ErrorKind::NotFound => ErrorCategory::InvalidImage,
        io::ErrorKind::Unsupported => ErrorCategory::UnsupportedFeature,
        io::ErrorKind::UnexpectedEof => ErrorCategory::FilesystemCorrupt,
        _ => ErrorCategory::FilesystemCorrupt,
    };
    Error::with_source(
        category,
        format!("SquashFS operation failed: {error}"),
        error,
    )
}
