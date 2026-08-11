use std::{error::Error as StdError, sync::Arc};

use ext4_view::{Ext4, Ext4Read};
use linuxfs_core::{BlockReader, Error, ErrorCategory, FilesystemInfo, Result};

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
        let _filesystem = Self::load(&self.reader)?;
        Ok(FilesystemInfo {
            filesystem_type: "ext2/ext3/ext4".to_owned(),
            label: None,
            uuid: None,
        })
    }

    fn load(reader: &Arc<dyn BlockReader>) -> Result<Ext4> {
        Ext4::load(Box::new(ReaderAdapter {
            reader: Arc::clone(reader),
        }))
        .map_err(|error| {
            Error::new(
                ErrorCategory::FilesystemCorrupt,
                format!("Ext filesystem rejected: {error}"),
            )
        })
    }
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
        assert_eq!(
            result.err().map(|error| error.category()),
            Some(ErrorCategory::FilesystemCorrupt)
        );
    }
}
