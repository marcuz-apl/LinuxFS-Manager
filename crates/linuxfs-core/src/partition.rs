use crate::{BlockGeometry, BlockReader, Error, ErrorCategory, Result};

const MBR_SIGNATURE_OFFSET: usize = 510;
const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLayout {
    DirectImage,
    Mbr { partitions: Vec<Partition> },
    Gpt { partitions: Vec<Partition> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub number: u32,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub type_identifier: PartitionType,
    pub unique_identifier: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionType {
    Mbr(u8),
    Gpt([u8; 16]),
}

pub fn discover_layout(reader: &dyn BlockReader, geometry: BlockGeometry) -> Result<SourceLayout> {
    let source_len = reader.len()?;
    if source_len < u64::from(crate::RAW_IMAGE_LOGICAL_SECTOR_SIZE) {
        return Ok(SourceLayout::DirectImage);
    }

    let sector_size = usize::try_from(geometry.logical_sector_size()).map_err(|_| {
        Error::new(
            ErrorCategory::InvalidImage,
            "logical sector size is too large",
        )
    })?;
    let sector_zero_len = if source_len < u64::from(geometry.logical_sector_size()) {
        usize::try_from(source_len).map_err(|_| {
            Error::new(
                ErrorCategory::InvalidImage,
                "source length does not fit usize",
            )
        })?
    } else {
        sector_size
    };
    let mut sector_zero = vec![0; sector_zero_len];
    reader.read_exact_at(0, &mut sector_zero)?;

    let has_mbr_signature = sector_zero.len() >= MBR_SIGNATURE_OFFSET + 2
        && sector_zero[MBR_SIGNATURE_OFFSET..MBR_SIGNATURE_OFFSET + 2] == [0x55, 0xAA];
    let has_gpt_signature = if source_len >= u64::from(geometry.logical_sector_size()) * 2 {
        let mut sector_one = vec![0; sector_size];
        reader.read_exact_at(u64::from(geometry.logical_sector_size()), &mut sector_one)?;
        sector_one.starts_with(GPT_SIGNATURE)
    } else {
        false
    };

    if has_mbr_signature || has_gpt_signature {
        return Err(Error::new(
            ErrorCategory::PartitionTable,
            "partition table signature requires validation",
        ));
    }
    Ok(SourceLayout::DirectImage)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryReader {
        bytes: Vec<u8>,
    }

    impl MemoryReader {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes }
        }
    }

    impl BlockReader for MemoryReader {
        fn len(&self) -> Result<u64> {
            Ok(self.bytes.len() as u64)
        }

        fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
            crate::validate_read_range(self.bytes.len() as u64, offset, destination.len())?;
            let start = usize::try_from(offset).map_err(|_| {
                Error::new(ErrorCategory::StorageAccess, "offset does not fit usize")
            })?;
            destination.copy_from_slice(&self.bytes[start..start + destination.len()]);
            Ok(())
        }
    }

    #[test]
    fn source_without_partition_signatures_is_direct_image() {
        let reader = MemoryReader::new(vec![0; 4096]);
        let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
            .expect("unsigned source is a direct image");
        assert!(matches!(layout, SourceLayout::DirectImage));
    }

    #[test]
    fn short_source_is_direct_image_without_out_of_range_read() {
        let reader = MemoryReader::new(vec![0; 100]);
        let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
            .expect("short source is a direct image");
        assert!(matches!(layout, SourceLayout::DirectImage));
    }

    #[test]
    fn partition_signature_fails_closed_until_validated() {
        let mut bytes = vec![0; 4096];
        bytes[510..512].copy_from_slice(&[0x55, 0xAA]);
        let error = discover_layout(&MemoryReader::new(bytes), BlockGeometry::raw_image_512())
            .expect_err("unvalidated table is rejected");
        assert_eq!(error.category(), ErrorCategory::PartitionTable);
    }

    #[test]
    fn gpt_signature_fails_closed_until_validated() {
        let mut bytes = vec![0; 4096];
        bytes[512..520].copy_from_slice(GPT_SIGNATURE);
        let error = discover_layout(&MemoryReader::new(bytes), BlockGeometry::raw_image_512())
            .expect_err("unvalidated GPT is rejected");
        assert_eq!(error.category(), ErrorCategory::PartitionTable);
    }
}
