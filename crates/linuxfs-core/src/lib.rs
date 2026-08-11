pub mod block;
pub mod error;

pub use block::{BlockGeometry, BlockReader, RAW_IMAGE_LOGICAL_SECTOR_SIZE, validate_read_range};
pub use error::{Error, ErrorCategory, Result};
