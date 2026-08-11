use std::collections::HashSet;

use super::{Partition, PartitionType};
use crate::{BlockGeometry, BlockReader, Error, ErrorCategory, Result};

const PARTITION_TABLE_OFFSET: usize = 446;
const PARTITION_ENTRY_SIZE: usize = 16;
const PRIMARY_ENTRY_COUNT: usize = 4;
const PROTECTIVE_GPT_TYPE: u8 = 0xEE;
const EXTENDED_TYPES: [u8; 3] = [0x05, 0x0F, 0x85];
const MAX_EBR_CHAIN_LENGTH: usize = 128;
const MBR_SIGNATURE: [u8; 2] = [0x55, 0xAA];

pub(super) fn parse(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    sector_zero: &[u8],
) -> Result<Vec<Partition>> {
    if sector_zero.len() < 512
        || sector_zero.len() < PARTITION_TABLE_OFFSET + PARTITION_ENTRY_SIZE * PRIMARY_ENTRY_COUNT
        || sector_zero[510..512] != MBR_SIGNATURE
    {
        return Err(table_error("invalid MBR sector"));
    }
    let mut partitions = Vec::new();
    let mut extended = None;
    for slot in 0..PRIMARY_ENTRY_COUNT {
        let entry = &sector_zero[PARTITION_TABLE_OFFSET + slot * PARTITION_ENTRY_SIZE
            ..PARTITION_TABLE_OFFSET + (slot + 1) * PARTITION_ENTRY_SIZE];
        let kind = entry[4];
        let start = u64::from(u32::from_le_bytes(
            entry[8..12]
                .try_into()
                .map_err(|_| table_error("invalid MBR entry"))?,
        ));
        let count = u64::from(u32::from_le_bytes(
            entry[12..16]
                .try_into()
                .map_err(|_| table_error("invalid MBR entry"))?,
        ));
        if kind == 0 {
            if start != 0 || count != 0 {
                return Err(table_error("empty MBR entry has a nonempty range"));
            }
            continue;
        }
        if kind == PROTECTIVE_GPT_TYPE {
            return Err(table_error("protective MBR requires GPT validation"));
        }
        if EXTENDED_TYPES.contains(&kind) {
            if extended.replace((start, count)).is_some() {
                return Err(table_error("multiple extended MBR containers"));
            }
            continue;
        }
        let (offset, length) = byte_range(reader, geometry, start, count)?;
        partitions.push(Partition {
            number: u32::try_from(slot + 1)
                .map_err(|_| table_error("partition number overflow"))?,
            byte_offset: offset,
            byte_length: length,
            type_identifier: PartitionType::Mbr(kind),
            unique_identifier: None,
        });
    }
    if let Some((base, count)) = extended {
        partitions.extend(parse_ebr_chain(reader, geometry, base, count)?);
    }
    Ok(partitions)
}

fn parse_ebr_chain(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    base: u64,
    count: u64,
) -> Result<Vec<Partition>> {
    let end = base
        .checked_add(count)
        .ok_or_else(|| table_error("extended MBR range overflow"))?;
    if count == 0 {
        return Err(table_error("empty extended MBR container"));
    }
    let _ = byte_range(reader, geometry, base, count)?;
    let mut current = base;
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    for index in 0..MAX_EBR_CHAIN_LENGTH {
        if !visited.insert(current) {
            return Err(table_error("repeated EBR offset"));
        }
        if current < base || current >= end {
            return Err(table_error("EBR lies outside extended container"));
        }
        let sector = read_sector(reader, geometry, current)?;
        if sector[510..512] != MBR_SIGNATURE {
            return Err(table_error("EBR signature is invalid"));
        }
        let first = read_entry(&sector, 0)?;
        let second = read_entry(&sector, 1)?;
        let third = read_entry(&sector, 2)?;
        let fourth = read_entry(&sector, 3)?;
        if !empty(third) || !empty(fourth) {
            return Err(table_error("unexpected extra EBR entries"));
        }
        if !empty(first) {
            if EXTENDED_TYPES.contains(&first.0) || first.0 == PROTECTIVE_GPT_TYPE || first.2 == 0 {
                return Err(table_error("invalid logical partition"));
            }
            let logical = current
                .checked_add(first.1)
                .ok_or_else(|| table_error("logical partition LBA overflow"))?;
            let logical_end = logical
                .checked_add(first.2)
                .ok_or_else(|| table_error("logical partition range overflow"))?;
            if logical < base || logical_end > end {
                return Err(table_error("logical partition escapes extended container"));
            }
            let (offset, length) = byte_range(reader, geometry, logical, first.2)?;
            result.push(Partition {
                number: 5 + u32::try_from(result.len())
                    .map_err(|_| table_error("logical partition number overflow"))?,
                byte_offset: offset,
                byte_length: length,
                type_identifier: PartitionType::Mbr(first.0),
                unique_identifier: None,
            });
        }
        if empty(first) && !empty(second) {
            return Err(table_error("EBR link has no logical partition"));
        }
        if empty(second) {
            return Ok(result);
        }
        if !EXTENDED_TYPES.contains(&second.0) || second.2 == 0 {
            return Err(table_error("invalid EBR next link"));
        }
        current = base
            .checked_add(second.1)
            .ok_or_else(|| table_error("next EBR LBA overflow"))?;
        if index + 1 == MAX_EBR_CHAIN_LENGTH {
            return Err(table_error("EBR chain exceeds limit"));
        }
    }
    Err(table_error("EBR chain exceeds limit"))
}

fn read_entry(sector: &[u8], index: usize) -> Result<(u8, u64, u64)> {
    let start = PARTITION_TABLE_OFFSET + index * PARTITION_ENTRY_SIZE;
    let bytes = sector
        .get(start..start + PARTITION_ENTRY_SIZE)
        .ok_or_else(|| table_error("EBR entry is truncated"))?;
    let kind = bytes[4];
    let lba = u64::from(u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| table_error("invalid EBR entry"))?,
    ));
    let count = u64::from(u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| table_error("invalid EBR entry"))?,
    ));
    Ok((kind, lba, count))
}

fn empty(entry: (u8, u64, u64)) -> bool {
    entry == (0, 0, 0)
}

fn read_sector(reader: &dyn BlockReader, geometry: BlockGeometry, lba: u64) -> Result<Vec<u8>> {
    let size = usize::try_from(geometry.logical_sector_size())
        .map_err(|_| table_error("sector size does not fit usize"))?;
    let offset = lba
        .checked_mul(u64::from(geometry.logical_sector_size()))
        .ok_or_else(|| table_error("sector offset overflow"))?;
    let mut sector = vec![0; size];
    reader
        .read_exact_at(offset, &mut sector)
        .map_err(|_| table_error("cannot read partition sector"))?;
    Ok(sector)
}

fn byte_range(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    start: u64,
    count: u64,
) -> Result<(u64, u64)> {
    if count == 0 {
        return Err(table_error("partition has zero sectors"));
    }
    let size = u64::from(geometry.logical_sector_size());
    let offset = start
        .checked_mul(size)
        .ok_or_else(|| table_error("partition offset overflow"))?;
    let length = count
        .checked_mul(size)
        .ok_or_else(|| table_error("partition length overflow"))?;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| table_error("partition range overflow"))?;
    if end
        > reader
            .len()
            .map_err(|_| table_error("cannot read source length"))?
    {
        return Err(table_error("partition extends beyond source"));
    }
    Ok((offset, length))
}

fn table_error(message: &'static str) -> Error {
    Error::new(ErrorCategory::PartitionTable, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockReader, SourceLayout};
    struct MemoryReader {
        bytes: Vec<u8>,
    }
    impl BlockReader for MemoryReader {
        fn len(&self) -> Result<u64> {
            Ok(self.bytes.len() as u64)
        }
        fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
            crate::validate_read_range(self.bytes.len() as u64, offset, destination.len())?;
            let start = usize::try_from(offset).map_err(|_| table_error("offset overflow"))?;
            destination.copy_from_slice(&self.bytes[start..start + destination.len()]);
            Ok(())
        }
    }
    fn image(sectors: usize) -> Vec<u8> {
        let mut bytes = vec![0; sectors * 512];
        bytes[510..512].copy_from_slice(&MBR_SIGNATURE);
        bytes
    }
    fn set_entry(bytes: &mut [u8], sector: usize, slot: usize, kind: u8, start: u32, count: u32) {
        let base = sector * 512 + PARTITION_TABLE_OFFSET + slot * PARTITION_ENTRY_SIZE;
        bytes[base + 4] = kind;
        bytes[base + 8..base + 12].copy_from_slice(&start.to_le_bytes());
        bytes[base + 12..base + 16].copy_from_slice(&count.to_le_bytes());
    }
    fn discover(bytes: Vec<u8>) -> Result<SourceLayout> {
        crate::discover_layout(&MemoryReader { bytes }, BlockGeometry::raw_image_512())
    }
    #[test]
    fn discovers_primary_linux_partition() {
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, 0x83, 2, 4);
        let layout = discover(bytes).expect("valid MBR parses");
        assert_eq!(
            layout,
            SourceLayout::Mbr {
                partitions: vec![Partition {
                    number: 1,
                    byte_offset: 1024,
                    byte_length: 2048,
                    type_identifier: PartitionType::Mbr(0x83),
                    unique_identifier: None
                }]
            }
        );
    }
    #[test]
    fn rejects_primary_partition_past_source_end() {
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, 0x83, 15, 2);
        assert_eq!(
            discover(bytes).expect_err("out of bounds").category(),
            ErrorCategory::PartitionTable
        );
    }
    #[test]
    fn rejects_inconsistent_empty_entry_and_protective_mbr() {
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, 0, 1, 0);
        assert_eq!(
            discover(bytes).expect_err("inconsistent").category(),
            ErrorCategory::PartitionTable
        );
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, PROTECTIVE_GPT_TYPE, 1, 14);
        assert_eq!(
            discover(bytes).expect_err("protective").category(),
            ErrorCategory::PartitionTable
        );
    }
    #[test]
    fn follows_two_logical_partitions() {
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, 0x05, 1, 8);
        set_entry(&mut bytes, 1, 0, 0x83, 1, 2);
        bytes[512 + 510..512 + 512].copy_from_slice(&MBR_SIGNATURE);
        set_entry(&mut bytes, 1, 1, 0x05, 4, 4);
        bytes[5 * 512 + 510..5 * 512 + 512].copy_from_slice(&MBR_SIGNATURE);
        set_entry(&mut bytes, 5, 0, 0x83, 1, 2);
        let SourceLayout::Mbr { partitions } = discover(bytes).expect("valid EBR chain") else {
            panic!("expected MBR")
        };
        assert_eq!(
            partitions.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![5, 6]
        );
        assert_eq!(partitions[0].byte_offset, 2 * 512);
        assert_eq!(partitions[1].byte_offset, 6 * 512);
    }
    #[test]
    fn rejects_repeated_ebr_offset() {
        let mut bytes = image(16);
        set_entry(&mut bytes, 0, 0, 0x05, 1, 8);
        set_entry(&mut bytes, 1, 0, 0x83, 1, 2);
        set_entry(&mut bytes, 1, 1, 0x05, 0, 8);
        assert_eq!(
            discover(bytes).expect_err("cycle").category(),
            ErrorCategory::PartitionTable
        );
    }
}
