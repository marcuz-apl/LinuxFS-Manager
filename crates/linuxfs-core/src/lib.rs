pub mod block;
pub mod error;
pub mod filesystem;
pub mod partition;

pub use block::{BlockGeometry, BlockReader, RAW_IMAGE_LOGICAL_SECTOR_SIZE, validate_read_range};
pub use error::{Error, ErrorCategory, Result};
pub use filesystem::{
    DirectoryEntry, FileKind, FilesystemInfo, FsPath, NodeMetadata, ReadOnlyFilesystem,
};
pub use partition::{Partition, PartitionType, SourceLayout, discover_layout};
