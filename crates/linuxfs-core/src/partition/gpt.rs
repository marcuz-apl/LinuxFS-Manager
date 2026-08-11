use super::{Partition, PartitionType, crc32};
use crate::{BlockGeometry, BlockReader, Error, ErrorCategory, Result};

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const GPT_REVISION_1_0: u32 = 0x0001_0000;
const MIN_HEADER_SIZE: usize = 92;
const MIN_ENTRY_SIZE: u32 = 128;
const MAX_ENTRY_SIZE: u32 = 4096;
const MAX_ENTRY_COUNT: u32 = 16_384;
const MAX_ENTRY_ARRAY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) struct Header {
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub entries_lba: u64,
    pub entry_count: u32,
    pub entry_size: u32,
    pub entries_crc32: u32,
}

pub(super) fn parse(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    header_sector: &[u8],
) -> Result<Vec<Partition>> {
    let source_len = reader
        .len()
        .map_err(|_| table_error("cannot read source length"))?;
    let header = parse_header(source_len, geometry, header_sector)?;
    let array_bytes = u64::from(header.entry_count)
        .checked_mul(u64::from(header.entry_size))
        .ok_or_else(|| table_error("GPT entry array overflow"))?;
    if array_bytes > MAX_ENTRY_ARRAY_BYTES {
        return Err(table_error("GPT entry array is too large"));
    }
    let array_len = usize::try_from(array_bytes)
        .map_err(|_| table_error("GPT entry array does not fit usize"))?;
    let sector_size = u64::from(geometry.logical_sector_size());
    let array_offset = header
        .entries_lba
        .checked_mul(sector_size)
        .ok_or_else(|| table_error("GPT entry array offset overflow"))?;
    let array_end = array_offset
        .checked_add(array_bytes)
        .ok_or_else(|| table_error("GPT entry array range overflow"))?;
    if array_end > source_len {
        return Err(table_error("GPT entry array extends beyond source"));
    }
    let mut entries = vec![0; array_len];
    reader
        .read_exact_at(array_offset, &mut entries)
        .map_err(|_| table_error("cannot read GPT entry array"))?;
    if crc32::ieee(&entries) != header.entries_crc32 {
        return Err(table_error("GPT entry array CRC mismatch"));
    }

    let mut partitions = Vec::new();
    let entry_size = usize::try_from(header.entry_size)
        .map_err(|_| table_error("GPT entry size does not fit usize"))?;
    for index in 0..header.entry_count {
        let index = usize::try_from(index).map_err(|_| table_error("GPT entry index overflow"))?;
        let start = index
            .checked_mul(entry_size)
            .ok_or_else(|| table_error("GPT entry offset overflow"))?;
        let entry = entries
            .get(start..start + entry_size)
            .ok_or_else(|| table_error("GPT entry is truncated"))?;
        if entry[..16].iter().all(|byte| *byte == 0) {
            continue;
        }
        let type_guid: [u8; 16] = entry[..16]
            .try_into()
            .map_err(|_| table_error("invalid GPT type GUID"))?;
        let unique_guid: [u8; 16] = entry[16..32]
            .try_into()
            .map_err(|_| table_error("invalid GPT unique GUID"))?;
        if unique_guid.iter().all(|byte| *byte == 0) {
            return Err(table_error("nonempty GPT entry has zero unique GUID"));
        }
        let first_lba = u64::from_le_bytes(
            entry[32..40]
                .try_into()
                .map_err(|_| table_error("invalid GPT first LBA"))?,
        );
        let last_lba = u64::from_le_bytes(
            entry[40..48]
                .try_into()
                .map_err(|_| table_error("invalid GPT last LBA"))?,
        );
        if first_lba > last_lba
            || first_lba < header.first_usable_lba
            || last_lba > header.last_usable_lba
        {
            return Err(table_error("GPT partition is outside usable range"));
        }
        let sector_count = last_lba
            .checked_sub(first_lba)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| table_error("GPT partition length overflow"))?;
        let byte_offset = first_lba
            .checked_mul(sector_size)
            .ok_or_else(|| table_error("GPT partition offset overflow"))?;
        let byte_length = sector_count
            .checked_mul(sector_size)
            .ok_or_else(|| table_error("GPT partition length overflow"))?;
        let byte_end = byte_offset
            .checked_add(byte_length)
            .ok_or_else(|| table_error("GPT partition range overflow"))?;
        if byte_end > source_len {
            return Err(table_error("GPT partition extends beyond source"));
        }
        partitions.push(Partition {
            number: u32::try_from(index + 1)
                .map_err(|_| table_error("GPT partition number overflow"))?,
            byte_offset,
            byte_length,
            type_identifier: PartitionType::Gpt(type_guid),
            unique_identifier: Some(unique_guid),
        });
    }
    Ok(partitions)
}

pub(super) fn parse_header(
    source_len: u64,
    geometry: BlockGeometry,
    sector: &[u8],
) -> Result<Header> {
    let sector_size = usize::try_from(geometry.logical_sector_size())
        .map_err(|_| table_error("sector size does not fit usize"))?;
    let sector_size_u64 = u64::from(geometry.logical_sector_size());
    if source_len == 0
        || !source_len.is_multiple_of(sector_size_u64)
        || sector.len() < sector_size
        || sector[..8] != *GPT_SIGNATURE
    {
        return Err(table_error("invalid GPT header sector"));
    }
    let revision = u32::from_le_bytes(
        sector[8..12]
            .try_into()
            .map_err(|_| table_error("invalid GPT revision"))?,
    );
    let header_size = usize::try_from(u32::from_le_bytes(
        sector[12..16]
            .try_into()
            .map_err(|_| table_error("invalid GPT header size"))?,
    ))
    .map_err(|_| table_error("GPT header size overflow"))?;
    let stored_crc = u32::from_le_bytes(
        sector[16..20]
            .try_into()
            .map_err(|_| table_error("invalid GPT header CRC"))?,
    );
    let reserved = u32::from_le_bytes(
        sector[20..24]
            .try_into()
            .map_err(|_| table_error("invalid GPT reserved field"))?,
    );
    let current_lba = u64::from_le_bytes(
        sector[24..32]
            .try_into()
            .map_err(|_| table_error("invalid GPT current LBA"))?,
    );
    let backup_lba = u64::from_le_bytes(
        sector[32..40]
            .try_into()
            .map_err(|_| table_error("invalid GPT backup LBA"))?,
    );
    let first_usable = u64::from_le_bytes(
        sector[40..48]
            .try_into()
            .map_err(|_| table_error("invalid GPT first usable LBA"))?,
    );
    let last_usable = u64::from_le_bytes(
        sector[48..56]
            .try_into()
            .map_err(|_| table_error("invalid GPT last usable LBA"))?,
    );
    let entries_lba = u64::from_le_bytes(
        sector[72..80]
            .try_into()
            .map_err(|_| table_error("invalid GPT entries LBA"))?,
    );
    let entry_count = u32::from_le_bytes(
        sector[80..84]
            .try_into()
            .map_err(|_| table_error("invalid GPT entry count"))?,
    );
    let entry_size = u32::from_le_bytes(
        sector[84..88]
            .try_into()
            .map_err(|_| table_error("invalid GPT entry size"))?,
    );
    let entries_crc32 = u32::from_le_bytes(
        sector[88..92]
            .try_into()
            .map_err(|_| table_error("invalid GPT entries CRC"))?,
    );
    let source_lbas = source_len / sector_size_u64;
    if revision != GPT_REVISION_1_0
        || reserved != 0
        || header_size < MIN_HEADER_SIZE
        || header_size > sector_size
        || current_lba != 1
        || backup_lba >= source_lbas
        || first_usable > last_usable
        || last_usable >= source_lbas
        || !(MIN_ENTRY_SIZE..=MAX_ENTRY_SIZE).contains(&entry_size)
        || entry_size % MIN_ENTRY_SIZE != 0
        || entry_count == 0
        || entry_count > MAX_ENTRY_COUNT
    {
        return Err(table_error("invalid GPT header fields"));
    }
    let array_bytes = u64::from(entry_count)
        .checked_mul(u64::from(entry_size))
        .ok_or_else(|| table_error("GPT entry array overflow"))?;
    if array_bytes > MAX_ENTRY_ARRAY_BYTES {
        return Err(table_error("GPT entry array is too large"));
    }
    let array_offset = entries_lba
        .checked_mul(sector_size_u64)
        .ok_or_else(|| table_error("GPT entry array offset overflow"))?;
    if array_offset
        .checked_add(array_bytes)
        .ok_or_else(|| table_error("GPT entry array range overflow"))?
        > source_len
    {
        return Err(table_error("GPT entry array extends beyond source"));
    }
    let mut crc_header = sector[..header_size].to_vec();
    crc_header[16..20].fill(0);
    if crc32::ieee(&crc_header) != stored_crc {
        return Err(table_error("GPT header CRC mismatch"));
    }
    Ok(Header {
        first_usable_lba: first_usable,
        last_usable_lba: last_usable,
        entries_lba,
        entry_count,
        entry_size,
        entries_crc32,
    })
}

fn table_error(message: &'static str) -> Error {
    Error::new(ErrorCategory::PartitionTable, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockGeometry;
    fn header_fixture() -> Vec<u8> {
        let mut sector = vec![0; 512];
        sector[..8].copy_from_slice(GPT_SIGNATURE);
        sector[8..12].copy_from_slice(&GPT_REVISION_1_0.to_le_bytes());
        sector[12..16].copy_from_slice(&92u32.to_le_bytes());
        sector[24..32].copy_from_slice(&1u64.to_le_bytes());
        sector[32..40].copy_from_slice(&127u64.to_le_bytes());
        sector[40..48].copy_from_slice(&34u64.to_le_bytes());
        sector[48..56].copy_from_slice(&126u64.to_le_bytes());
        sector[72..80].copy_from_slice(&2u64.to_le_bytes());
        sector[80..84].copy_from_slice(&1u32.to_le_bytes());
        sector[84..88].copy_from_slice(&128u32.to_le_bytes());
        sector[88..92].copy_from_slice(&crc32::ieee(&[0; 128]).to_le_bytes());
        let crc = crc32::ieee(&sector[..92]);
        sector[16..20].copy_from_slice(&crc.to_le_bytes());
        sector
    }
    struct MemoryReader {
        bytes: Vec<u8>,
    }
    impl crate::BlockReader for MemoryReader {
        fn len(&self) -> crate::Result<u64> {
            Ok(self.bytes.len() as u64)
        }
        fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> crate::Result<()> {
            crate::validate_read_range(self.bytes.len() as u64, offset, destination.len())?;
            let start = usize::try_from(offset).map_err(|_| table_error("offset overflow"))?;
            destination.copy_from_slice(&self.bytes[start..start + destination.len()]);
            Ok(())
        }
    }

    fn one_partition_image() -> MemoryReader {
        let mut bytes = vec![0; 128 * 512];
        let mut entries = vec![0; 128];
        for (index, byte) in entries[..16].iter_mut().enumerate() {
            *byte = (index + 1) as u8;
        }
        for (index, byte) in entries[16..32].iter_mut().enumerate() {
            *byte = (index + 17) as u8;
        }
        entries[32..40].copy_from_slice(&40u64.to_le_bytes());
        entries[40..48].copy_from_slice(&41u64.to_le_bytes());
        bytes[2 * 512..2 * 512 + 128].copy_from_slice(&entries);
        let mut header = header_fixture();
        header[88..92].copy_from_slice(&crc32::ieee(&entries).to_le_bytes());
        header[16..20].fill(0);
        let header_crc = crc32::ieee(&header[..92]);
        header[16..20].copy_from_slice(&header_crc.to_le_bytes());
        bytes[512..1024].copy_from_slice(&header);
        MemoryReader { bytes }
    }

    #[test]
    fn discovers_valid_gpt_partition() {
        let layout = crate::discover_layout(&one_partition_image(), BlockGeometry::raw_image_512())
            .expect("valid GPT parses");
        let crate::SourceLayout::Gpt { partitions } = layout else {
            panic!("expected GPT layout")
        };
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].number, 1);
        assert_eq!(partitions[0].byte_offset, 40 * 512);
        assert_eq!(partitions[0].byte_length, 2 * 512);
        assert!(matches!(
            partitions[0].type_identifier,
            PartitionType::Gpt(_)
        ));
        assert!(partitions[0].unique_identifier.is_some());
    }
    #[test]
    fn accepts_valid_header() {
        let sector = header_fixture();
        assert!(parse_header(128 * 512, BlockGeometry::raw_image_512(), &sector).is_ok());
    }
    #[test]
    fn rejects_header_crc_corruption() {
        let mut sector = header_fixture();
        sector[40] ^= 1;
        assert_eq!(
            parse_header(128 * 512, BlockGeometry::raw_image_512(), &sector)
                .expect_err("CRC")
                .category(),
            ErrorCategory::PartitionTable
        );
    }
    #[test]
    fn rejects_invalid_header_limits() {
        let mut sector = header_fixture();
        sector[12..16].copy_from_slice(&91u32.to_le_bytes());
        assert!(parse_header(128 * 512, BlockGeometry::raw_image_512(), &sector).is_err());
        let mut sector = header_fixture();
        sector[84..88].copy_from_slice(&127u32.to_le_bytes());
        assert!(parse_header(128 * 512, BlockGeometry::raw_image_512(), &sector).is_err());
    }
}
