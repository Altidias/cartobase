pub const CONTAINER_CHUNKS: i32 = 32;
pub const CONTAINER_BITS: usize = (CONTAINER_CHUNKS * CONTAINER_CHUNKS) as usize;
pub const CONTAINER_BYTES: usize = CONTAINER_BITS / 8;

pub fn container_of(cx: i32, cz: i32) -> (i32, i32) {
    (cx.div_euclid(CONTAINER_CHUNKS), cz.div_euclid(CONTAINER_CHUNKS))
}

fn local_index(cx: i32, cz: i32) -> usize {
    let lx = cx.rem_euclid(CONTAINER_CHUNKS) as usize;
    let lz = cz.rem_euclid(CONTAINER_CHUNKS) as usize;
    lz * CONTAINER_CHUNKS as usize + lx
}

// sets the bit for (cx,cz); returns true if it was previously unset
pub fn set_bit(bitmap: &mut [u8], cx: i32, cz: i32) -> bool {
    let idx = local_index(cx, cz);
    let byte = idx / 8;
    let mask = 1u8 << (idx % 8);
    let was = bitmap[byte] & mask != 0;
    bitmap[byte] |= mask;
    !was
}

pub fn popcount(bitmap: &[u8]) -> i32 {
    bitmap.iter().map(|b| b.count_ones()).sum::<u32>() as i32
}

// yields absolute chunk coords for every set bit in a container
pub fn decode_container(container_x: i32, container_z: i32, bitmap: &[u8], out: &mut Vec<[i32; 2]>) {
    for idx in 0..CONTAINER_BITS {
        if bitmap[idx / 8] & (1u8 << (idx % 8)) == 0 {
            continue;
        }
        let lx = (idx % CONTAINER_CHUNKS as usize) as i32;
        let lz = (idx / CONTAINER_CHUNKS as usize) as i32;
        out.push([container_x * CONTAINER_CHUNKS + lx, container_z * CONTAINER_CHUNKS + lz]);
    }
}

pub fn empty_bitmap() -> Vec<u8> {
    vec![0u8; CONTAINER_BYTES]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn roundtrip_including_negatives() {
        let coords = [
            [0, 0], [1, 0], [0, 1], [31, 31], [32, 0], [-1, -1], [-33, 5], [1000, -2000],
        ];
        let mut containers: HashMap<(i32, i32), Vec<u8>> = HashMap::new();
        for c in coords {
            let key = container_of(c[0], c[1]);
            let bm = containers.entry(key).or_insert_with(empty_bitmap);
            assert!(set_bit(bm, c[0], c[1]));
            assert!(!set_bit(bm, c[0], c[1]));
        }
        let mut decoded = Vec::new();
        let mut total = 0;
        for ((cx, cz), bm) in &containers {
            total += popcount(bm);
            decode_container(*cx, *cz, bm, &mut decoded);
        }
        assert_eq!(total as usize, coords.len());
        decoded.sort();
        let mut expected: Vec<[i32; 2]> = coords.to_vec();
        expected.sort();
        assert_eq!(decoded, expected);
    }
}
