pub fn compute_checksum(data: &Vec<u8>) -> u16 {
    let mut sum: u32 = 0;

    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum += word;

        if sum > 0xFFFF {
            sum = (sum & 0xFFFF) + 1;
        }
    }

    if let Some(&last) = chunks.remainder().get(0) {
        let word = (last as u16) << 8;
        sum += word as u32;

        if sum > 0xFFFF {
            sum = (sum & 0xFFFF) + 1;
        }
    }

    !(sum as u16)
}


pub fn verify_checksum(data: Vec<u8>, checksum: u16) -> bool {
    let mut sum: u32 = 0;

    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum += word;

        if sum > 0xFFFF {
            sum = (sum & 0xFFFF) + 1;
        }
    }

    if let Some(&last) = chunks.remainder().get(0) {
        let word = (last as u16) << 8;
        sum += word as u32;

        if sum > 0xFFFF {
            sum = (sum & 0xFFFF) + 1;
        }
    }

    sum += checksum as u32;
    if sum > 0xFFFF {
        sum = (sum & 0xFFFF) + 1;
    }

    sum == 0xFFFF
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_valid() {
        let data = b"bebebe bababa((";

        let checksum = compute_checksum(&Vec::from(data));
        assert!(verify_checksum(Vec::from(data), checksum));
    }

    #[test]
    fn test_checksum_invalid() {
        let data = b"bebebe bababa))";
        let mut corrupted = data.to_vec();

        corrupted[0] ^= 0xFF;

        let checksum = compute_checksum(&Vec::from(data));
        assert!(!verify_checksum(corrupted, checksum));
    }

    #[test]
    fn test_checksum_collision() {
        let data1 = [0x00u8, 0x01, 0x00, 0x02];
        let data2 = [0x00u8, 0x02, 0x00, 0x01];

        let checksum = compute_checksum(&Vec::from(data1));

        assert!(verify_checksum(Vec::from(data2), checksum));

        assert_ne!(data1, data2);
    }
}