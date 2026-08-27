use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub enum CborError {
    NonMinimalEncoding,
    ForbiddenFloat,
    ForbiddenTag,
    IndefiniteLength,
    DuplicateMapKey,
    NonCanonicalMapOrder,
    TrailingBytes,
    TruncatedData,
    InvalidUtf8,
    ForbiddenBreak,
    InvalidSimpleValue,
    ResourceExhaustion,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SchemaError {
    InvalidSchema,
    InvalidHashRef,
    InvalidSignatureEnvelope,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConsensusError {
    InvalidAuthority,
    GenesisCollision,
    InvalidCapabilityCycle,
    UnauthorizedCapability,
    InvalidDescendant,
    Unresolved,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NexParseError {
    Cbor(CborError),
    Schema(SchemaError),
    Consensus(ConsensusError),
}

impl From<CborError> for NexParseError {
    fn from(err: CborError) -> Self { NexParseError::Cbor(err) }
}
impl From<SchemaError> for NexParseError {
    fn from(err: SchemaError) -> Self { NexParseError::Schema(err) }
}
impl From<ConsensusError> for NexParseError {
    fn from(err: ConsensusError) -> Self { NexParseError::Consensus(err) }
}

const MAX_DEPTH: usize = 64;
const MAX_CONTAINER_ITEMS: usize = 65536;
const MAX_TOTAL_ITEMS: usize = 256_000;
const MAX_TOTAL_BYTES: usize = 16_777_216; // 16 MB

pub struct NexCborValidator {
    total_items: usize,
    total_bytes: usize,
}

impl NexCborValidator {
    pub fn validate(raw_bytes: &[u8]) -> Result<(), CborError> {
        if raw_bytes.len() > MAX_TOTAL_BYTES {
            return Err(CborError::ResourceExhaustion);
        }
        let mut validator = NexCborValidator { total_items: 0, total_bytes: raw_bytes.len() };
        let consumed = validator.validate_item(raw_bytes, 0)?;
        if consumed != raw_bytes.len() {
            return Err(CborError::TrailingBytes);
        }
        Ok(())
    }

    fn validate_item(&mut self, bytes: &[u8], depth: usize) -> Result<usize, CborError> {
        if depth > MAX_DEPTH { return Err(CborError::ResourceExhaustion); }
        self.total_items += 1;
        if self.total_items > MAX_TOTAL_ITEMS { return Err(CborError::ResourceExhaustion); }
        if bytes.is_empty() { return Err(CborError::TruncatedData); }

        let first = bytes[0];
        let major = first >> 5;
        let info = first & 0x1F;

        if info == 31 {
            if major == 7 { return Err(CborError::ForbiddenBreak); }
            return Err(CborError::IndefiniteLength);
        }
        if major == 6 { return Err(CborError::ForbiddenTag); }
        if major == 7 {
            match info {
                25 | 26 | 27 => return Err(CborError::ForbiddenFloat),
                24 => {
                    if bytes.len() < 2 { return Err(CborError::TruncatedData); }
                    if bytes[1] < 32 { return Err(CborError::NonMinimalEncoding); }
                },
                _ => {}
            }
        }

        let (arg, arg_bytes_len) = Self::parse_argument(bytes)?;
        let mut offset = arg_bytes_len;

        match major {
            0 | 1 => Ok(offset),
            2 | 3 => {
                let len = arg as usize;
                if offset.checked_add(len).map_or(true, |end| end > bytes.len()) {
                    return Err(CborError::TruncatedData);
                }
                if major == 3 && std::str::from_utf8(&bytes[offset..offset + len]).is_err() {
                    return Err(CborError::InvalidUtf8);
                }
                Ok(offset + len)
            }
            4 => {
                let count = arg as usize;
                if count > MAX_CONTAINER_ITEMS { return Err(CborError::ResourceExhaustion); }
                for _ in 0..count {
                    let item_len = self.validate_item(&bytes[offset..], depth + 1)?;
                    offset += item_len;
                }
                Ok(offset)
            }
            5 => {
                let count = arg as usize;
                if count > MAX_CONTAINER_ITEMS { return Err(CborError::ResourceExhaustion); }
                let mut last_key: Option<&[u8]> = None;

                for _ in 0..count {
                    let key_start = offset;
                    let key_len = self.validate_item(&bytes[offset..], depth + 1)?;
                    let key_slice = &bytes[key_start..key_start + key_len];
                    offset += key_len;

                    if let Some(prev_key) = last_key {
                        match key_slice.cmp(prev_key) {
                            Ordering::Less => return Err(CborError::NonCanonicalMapOrder),
                            Ordering::Equal => return Err(CborError::DuplicateMapKey),
                            Ordering::Greater => {}
                        }
                    }
                    last_key = Some(key_slice);

                    let val_len = self.validate_item(&bytes[offset..], depth + 1)?;
                    offset += val_len;
                }
                Ok(offset)
            }
            7 => Ok(offset),
            _ => unreachable!(),
        }
    }

    fn parse_argument(bytes: &[u8]) -> Result<(u64, usize), CborError> {
        let info = bytes[0] & 0x1F;
        match info {
            v if v < 24 => Ok((v as u64, 1)),
            24 => {
                if bytes.len() < 2 { return Err(CborError::TruncatedData); }
                let val = bytes[1] as u64;
                if val < 24 { return Err(CborError::NonMinimalEncoding); }
                Ok((val, 2))
            }
            25 => {
                if bytes.len() < 3 { return Err(CborError::TruncatedData); }
                let val = u16::from_be_bytes([bytes[1], bytes[2]]) as u64;
                if val <= 0xFF { return Err(CborError::NonMinimalEncoding); }
                Ok((val, 3))
            }
            26 => {
                if bytes.len() < 5 { return Err(CborError::TruncatedData); }
                let val = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                if val <= 0xFFFF { return Err(CborError::NonMinimalEncoding); }
                Ok((val, 5))
            }
            27 => {
                if bytes.len() < 9 { return Err(CborError::TruncatedData); }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[1..9]);
                let val = u64::from_be_bytes(buf);
                if val <= 0xFFFFFFFF { return Err(CborError::NonMinimalEncoding); }
                Ok((val, 9))
            }
            _ => Err(CborError::InvalidSimpleValue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_structures() {
        assert_eq!(NexCborValidator::validate(&[0x00]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0x18, 0x18]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0x38, 0x18]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0x40]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0x60]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0x80]), Ok(())); 
        assert_eq!(NexCborValidator::validate(&[0xA0]), Ok(())); 
    }

    #[test]
    fn test_encoded_key_ordering() {
        // Correctly ordered:
        // 0x0a (10) < 0x18 0x64 (100) < 0x20 ("") < 0x61 0x7a ("z") < 0x62 0x61 0x61 ("aa")
        let correct = [
            0xA5,
            0x0A, 0x01,                   // 10: 1
            0x18, 0x64, 0x01,             // 100: 1
            0x20, 0x01,                   // b"": 1
            0x61, 0x7A, 0x01,             // "z": 1
            0x62, 0x61, 0x61, 0x01        // "aa": 1
        ];
        assert_eq!(NexCborValidator::validate(&correct), Ok(()));

        // Incorrectly ordered (length-first or arbitrary): "aa" before "z"
        let incorrect = [
            0xA5,
            0x0A, 0x01,
            0x18, 0x64, 0x01,
            0x20, 0x01,
            0x62, 0x61, 0x61, 0x01,       // "aa": 1
            0x61, 0x7A, 0x01              // "z": 1
        ];
        assert_eq!(NexCborValidator::validate(&incorrect), Err(CborError::NonCanonicalMapOrder));
    }

    #[test]
    fn test_metamorphic_ordering() {
        // {1: 1, 2: 2, 3: 3}
        let abc = [0xA3, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03];
        assert_eq!(NexCborValidator::validate(&abc), Ok(()));
        
        let acb = [0xA3, 0x01, 0x01, 0x03, 0x03, 0x02, 0x02];
        let bac = [0xA3, 0x02, 0x02, 0x01, 0x01, 0x03, 0x03];
        assert_eq!(NexCborValidator::validate(&acb), Err(CborError::NonCanonicalMapOrder));
        assert_eq!(NexCborValidator::validate(&bac), Err(CborError::NonCanonicalMapOrder));
    }

    #[test]
    fn test_nested_duplicate_keys() {
        // { 1: { 1: A, 1: B } }
        let bytes = [
            0xA1, // map(1)
            0x01, // key 1
            0xA2, // map(2)
            0x01, 0x61, 0x41, // 1: "A"
            0x01, 0x61, 0x42, // 1: "B"
        ];
        assert_eq!(NexCborValidator::validate(&bytes), Err(CborError::DuplicateMapKey));
    }

    #[test]
    fn test_simple_values() {
        assert_eq!(NexCborValidator::validate(&[0xF4]), Ok(())); // false
        assert_eq!(NexCborValidator::validate(&[0xF5]), Ok(())); // true
        assert_eq!(NexCborValidator::validate(&[0xF6]), Ok(())); // null
        assert_eq!(NexCborValidator::validate(&[0xF7]), Ok(())); // undefined
        assert_eq!(NexCborValidator::validate(&[0xF8, 0x20]), Ok(())); // extended simple value 32

        // Non-minimal simple value (e.g. 23 encoded as 0xF8 0x17)
        assert_eq!(NexCborValidator::validate(&[0xF8, 0x17]), Err(CborError::NonMinimalEncoding));

        // Break outside indefinite container
        assert_eq!(NexCborValidator::validate(&[0xFF]), Err(CborError::ForbiddenBreak));
    }

    #[test]
    fn test_truncated_data() {
        assert_eq!(NexCborValidator::validate(&[0x82, 0x01]), Err(CborError::TruncatedData));
    }

    #[test]
    fn test_trailing_bytes() {
        assert_eq!(NexCborValidator::validate(&[0x80, 0x00]), Err(CborError::TrailingBytes));
    }

    #[test]
    fn test_nested_forbidden() {
        // array containing a float
        let bytes = [0x81, 0xFA, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(NexCborValidator::validate(&bytes), Err(CborError::ForbiddenFloat));
    }

    #[test]
    fn test_resource_limits() {
        // Test MAX_TOTAL_BYTES
        let huge_bytes = vec![0x00; 16_777_217]; // 1 byte over 16MB
        assert_eq!(NexCborValidator::validate(&huge_bytes), Err(CborError::ResourceExhaustion));

        // Test MAX_DEPTH
        // 65 nested arrays = 65 bytes of 0x81 (array of 1), followed by 0x00
        let mut deep = vec![0x81; 65];
        deep.push(0x00);
        assert_eq!(NexCborValidator::validate(&deep), Err(CborError::ResourceExhaustion));

        let mut safe_deep = vec![0x81; 64];
        safe_deep.push(0x00);
        assert_eq!(NexCborValidator::validate(&safe_deep), Ok(()));

        // Test MAX_CONTAINER_ITEMS
        // Array of 65537 items (requires 4-byte length)
        // 0x9A 0x00 0x01 0x00 0x01 = Array of length 65537
        let mut large_array = vec![0x9A, 0x00, 0x01, 0x00, 0x01];
        large_array.extend(vec![0x00; 65537]); // Fill with 0s
        assert_eq!(NexCborValidator::validate(&large_array), Err(CborError::ResourceExhaustion));
    }
}
