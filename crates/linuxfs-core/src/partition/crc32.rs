pub(super) fn ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_ieee_known_vectors() {
        assert_eq!(ieee(b""), 0x0000_0000);
        assert_eq!(ieee(b"123456789"), 0xCBF4_3926);
    }
}
