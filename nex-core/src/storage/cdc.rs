use sha2::{Sha256, Digest};

pub const DEFAULT_MIN_CHUNK_SIZE: usize = 16 * 1024;   // 16 KB
pub const DEFAULT_AVG_CHUNK_SIZE: usize = 64 * 1024;   // 64 KB
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 256 * 1024;  // 256 KB

// FastCDC Gear Matrix for fast rolling hash
const GEAR_TABLE: [u32; 256] = [
    0x8a970e5b, 0xbc9f4568, 0x3211538b, 0x58ac37a7, 0xcab25c15, 0x3c479541, 0x14a59159, 0x44c7d1ef,
    0x28052679, 0x3b611461, 0x8da6d796, 0xb6464978, 0xa7d87954, 0x72ddda13, 0x9de9da84, 0xba69e6b2,
    0x237537d5, 0x5ef77fb2, 0x6e1150bc, 0x40e9402d, 0x192ca5d6, 0xcd0d78e7, 0xd0e9aa5e, 0x455c79dc,
    0x08a73a35, 0x56a14b51, 0x5214d02c, 0x717814a4, 0x5d5030f1, 0xd2a4679f, 0x8269dad7, 0x90a9e0e6,
    0xa4b7bcdc, 0xb2813c4e, 0x7947ff3a, 0xba39b64b, 0xaa76fd82, 0xbda63a46, 0xb003be4d, 0x40515603,
    0xbf59be37, 0xb01e1570, 0x137e3be4, 0xd567d130, 0xfa25920b, 0xab572dbd, 0xd308017f, 0x67c354d3,
    0x886564cc, 0x406d02d3, 0x35ecd2b3, 0xa08404a5, 0x6782d2a2, 0xcebbf241, 0xf7613135, 0x16be8b65,
    0x22e4ff73, 0x0f0fefee, 0xb0524b01, 0x06be630a, 0xa0a270cc, 0x685f6bb7, 0x9f0d3efb, 0xb078b8ff,
    0x126747f2, 0xcbba84a4, 0x8fb81b9f, 0xae5a5fb8, 0x6d113cce, 0x83824e0e, 0x805299f7, 0x894384b9,
    0x53c4b571, 0x4e29cb14, 0xa371d6d2, 0x4460d41a, 0x63459fe0, 0xdefc04c4, 0x47b99e7a, 0x4fc5ba3e,
    0xa8f50cbd, 0x3c910314, 0x6e120198, 0xa9108297, 0x12839d9f, 0x0b451340, 0x9d02add1, 0x786d05cd,
    0x48420793, 0xac08b732, 0x7ced8e10, 0xbe236905, 0xab87ff31, 0x35e00f1c, 0x7d93be02, 0x7a6c229c,
    0x8cf87eed, 0xb69c3ff8, 0x83bf4b01, 0x7d62b606, 0x03fb5701, 0x381285b0, 0x8d21fd10, 0x3d2862b2,
    0xf89ee50f, 0x256f25e2, 0x72922620, 0xaebe5f81, 0xd5474244, 0x5ac52716, 0x43fb4415, 0x77d01140,
    0x50c0e7fe, 0x4ce14164, 0xda490f2b, 0x0d676f9b, 0x356bc611, 0x7e4e1f16, 0x42fc97a3, 0x737d9967,
    0x61de7cac, 0xa05c1bda, 0x47396644, 0xdc86e08a, 0x5b6f215a, 0x8308520f, 0x073a4f8b, 0x816b7045,
    0x5352ab1e, 0xd6ae7b38, 0x2b209304, 0xed896942, 0x7b110080, 0xd1b44518, 0x706c6045, 0xb343a8b0,
    0xb4764403, 0x8d6ec441, 0x663322b1, 0x743522f2, 0xd1e03b49, 0xa934ee35, 0x88342084, 0xea34ee2d,
    0x49a11470, 0x524fe345, 0x68205296, 0x38680103, 0x263d3d9d, 0x59c02500, 0x3785b440, 0xa8ea4142,
    0x3cdbe49d, 0x869713e2, 0x55b0872e, 0x43b59241, 0x11356ee2, 0x28615123, 0x5822292b, 0xda0c30e0,
    0x319234b2, 0x6522aa24, 0xb3e22345, 0xd46e2202, 0x48225092, 0xa6291220, 0x3060182b, 0xd11f0ee0,
    0x6ed41b01, 0x47bf5904, 0x1313e406, 0xeb63e003, 0x5026ff00, 0x64cebe85, 0xe1f009f0, 0xd0e42f0b,
    0x2512a3bc, 0xe47a10f4, 0x05210a04, 0x0114220e, 0x83334c6d, 0x3e240b08, 0x52b00045, 0x4b290d9e,
    0x8544e390, 0xb955ea08, 0xb28a3016, 0x36716088, 0x6722d540, 0x02888a76, 0xb0fe880c, 0x61e4418f,
    0xce512fec, 0x200a1301, 0x10ae348b, 0xd06acbfe, 0x3e450502, 0xe114ee11, 0x0ce040e1, 0x515fe04f,
    0xa1547ce0, 0x925050f2, 0x65b8205e, 0xb28202fe, 0x9b2630ee, 0xebd00b4a, 0x45fe8020, 0x83d642a0,
    0xfca12002, 0x61450008, 0xf125bb50, 0x49ac3415, 0x3b220042, 0xc1008500, 0x97ee56f1, 0xb433005e,
    0x082613b0, 0x49531105, 0x03e5e4aa, 0xc7798820, 0x31b40280, 0xd6fc1214, 0x2b3450e0, 0xd1e09990,
    0x4422fe12, 0x2a045b15, 0x20b41220, 0xae120088, 0xb16244f0, 0xb972408b, 0xee21b012, 0x13be50fe,
    0xa0872200, 0x86241b11, 0xb25fe01c, 0x31ae3008, 0x4c24e0fe, 0x8b31201b, 0x81ee4504, 0xfa312b00,
    0xa25e003c, 0xac421105, 0x43b22002, 0x44fe0211, 0x0ceb2040, 0xe01200fe, 0xc0fe112b, 0x3f51000b,
    0xd054ee40, 0x225100b2, 0x5140ee11, 0xb05e45fe, 0x4f0e0b11, 0xf0204b15, 0x854fe00b, 0x90e4b011,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkBoundary {
    pub offset: usize,
    pub length: usize,
    pub chunk_hash: [u8; 32],
}

pub struct FastCdcChunker {
    min_size: usize,
    avg_size: usize,
    max_size: usize,
    mask_small: u32,
    mask_large: u32,
}

impl Default for FastCdcChunker {
    fn default() -> Self {
        Self::new(DEFAULT_MIN_CHUNK_SIZE, DEFAULT_AVG_CHUNK_SIZE, DEFAULT_MAX_CHUNK_SIZE)
    }
}

fn compute_mask(target_size: usize) -> u32 {
    let bits = (target_size as f64).log2().round() as u32;
    if bits >= 32 {
        0xFFFF_FFFF
    } else {
        (1 << bits) - 1
    }
}

impl FastCdcChunker {
    pub fn new(min_size: usize, avg_size: usize, max_size: usize) -> Self {
        assert!(min_size <= avg_size, "min_size must be <= avg_size");
        assert!(avg_size <= max_size, "avg_size must be <= max_size");
        
        let mask = compute_mask(avg_size);
        let mask_small = mask | (mask >> 2);
        let mask_large = mask & !(mask >> 2);

        Self {
            min_size,
            avg_size,
            max_size,
            mask_small,
            mask_large,
        }
    }

    /// Chunks arbitrary byte slice into content-defined variable chunks
    pub fn chunk_slice(&self, data: &[u8]) -> Vec<ChunkBoundary> {
        let mut boundaries = Vec::new();
        let total_len = data.len();
        if total_len == 0 {
            return boundaries;
        }

        let mut offset = 0;

        while offset < total_len {
            let remaining = total_len - offset;
            if remaining <= self.min_size {
                let chunk_data = &data[offset..total_len];
                let mut hasher = Sha256::new();
                hasher.update(chunk_data);
                boundaries.push(ChunkBoundary {
                    offset,
                    length: remaining,
                    chunk_hash: hasher.finalize().into(),
                });
                break;
            }

            let max_possible = remaining.min(self.max_size);
            let mut finger_print: u32 = 0;
            let mut chunk_len = self.min_size;

            while chunk_len < max_possible {
                let byte = data[offset + chunk_len];
                finger_print = (finger_print << 1).wrapping_add(GEAR_TABLE[byte as usize]);

                let mask = if chunk_len < self.avg_size {
                    self.mask_small
                } else {
                    self.mask_large
                };

                if (finger_print & mask) == 0 {
                    chunk_len += 1;
                    break;
                }
                chunk_len += 1;
            }

            let chunk_data = &data[offset..offset + chunk_len];
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            boundaries.push(ChunkBoundary {
                offset,
                length: chunk_len,
                chunk_hash: hasher.finalize().into(),
            });

            offset += chunk_len;
        }

        boundaries
    }

    /// Verifies that all chunk boundaries exactly reconstruct the original data
    pub fn verify_integrity(data: &[u8], boundaries: &[ChunkBoundary]) -> bool {
        let mut current_offset = 0;
        for b in boundaries {
            if b.offset != current_offset {
                return false;
            }
            if current_offset + b.length > data.len() {
                return false;
            }
            let chunk_data = &data[b.offset..b.offset + b.length];
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash: [u8; 32] = hasher.finalize().into();
            if hash != b.chunk_hash {
                return false;
            }
            current_offset += b.length;
        }
        current_offset == data.len()
    }
}
