use crate::error::{Error, ErrorCategory, Result};

pub const RAW_IMAGE_LOGICAL_SECTOR_SIZE: u32 = 512;
const MAX_LOGICAL_SECTOR_SIZE: u32 = 65_536;

#[allow(clippy::len_without_is_empty)]
pub trait BlockReader: Send + Sync {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockGeometry {
    logical_sector_size: u32,
}

impl BlockGeometry {
    pub fn new(logical_sector_size: u32) -> Result<Self> {
        if !(RAW_IMAGE_LOGICAL_SECTOR_SIZE..=MAX_LOGICAL_SECTOR_SIZE).contains(&logical_sector_size)
            || !logical_sector_size.is_power_of_two()
        {
            return Err(Error::new(
                ErrorCategory::InvalidImage,
                "unsupported logical sector size",
            ));
        }
        Ok(Self {
            logical_sector_size,
        })
    }

    pub const fn raw_image_512() -> Self {
        Self {
            logical_sector_size: 512,
        }
    }

    pub const fn logical_sector_size(self) -> u32 {
        self.logical_sector_size
    }
}

pub fn validate_read_range(source_len: u64, offset: u64, requested_len: usize) -> Result<()> {
    let requested_len = u64::try_from(requested_len)
        .map_err(|_| Error::new(ErrorCategory::StorageAccess, "read length does not fit u64"))?;
    let end = offset
        .checked_add(requested_len)
        .ok_or_else(|| Error::new(ErrorCategory::StorageAccess, "read range overflow"))?;
    if end > source_len {
        return Err(Error::new(
            ErrorCategory::StorageAccess,
            "read extends beyond source",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_accepts_supported_sector_sizes() {
        let sector_512 = BlockGeometry::new(512).expect("512-byte sectors are supported");
        let sector_4096 = BlockGeometry::new(4096).expect("4096-byte sectors are supported");
        assert_eq!(sector_512.logical_sector_size(), 512);
        assert_eq!(sector_4096.logical_sector_size(), 4096);
    }

    #[test]
    fn geometry_rejects_zero_non_power_of_two_and_excessive_values() {
        for value in [0, 513, 131_072] {
            assert_eq!(
                BlockGeometry::new(value).map_err(|error| error.category()),
                Err(ErrorCategory::InvalidImage)
            );
        }
    }

    #[test]
    fn read_range_rejects_overflow_and_end_past_source() {
        assert!(validate_read_range(16, u64::MAX, 1).is_err());
        assert!(validate_read_range(16, 15, 2).is_err());
        assert!(validate_read_range(16, 16, 0).is_ok());
    }
}

use std::sync::Arc;

/// A bounded read-only view over a region of another block source.
#[derive(Clone)]
pub struct PartitionReader {
    source: Arc<dyn BlockReader>,
    offset: u64,
    length: u64,
}

impl PartitionReader {
    pub fn new(source: Arc<dyn BlockReader>, offset: u64, length: u64) -> Result<Self> {
        let source_len = source.len()?;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| Error::new(ErrorCategory::InvalidImage, "partition range overflow"))?;
        if end > source_len {
            return Err(Error::new(
                ErrorCategory::InvalidImage,
                "partition extends beyond source",
            ));
        }
        Ok(Self {
            source,
            offset,
            length,
        })
    }
}

impl BlockReader for PartitionReader {
    fn len(&self) -> Result<u64> {
        Ok(self.length)
    }

    fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
        validate_read_range(self.length, offset, destination.len())?;
        let absolute = self.offset.checked_add(offset).ok_or_else(|| {
            Error::new(
                ErrorCategory::StorageAccess,
                "partition read offset overflow",
            )
        })?;
        self.source.read_exact_at(absolute, destination)
    }
}
