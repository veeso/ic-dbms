use crate::memory::error::DecodeError;
use crate::memory::{Encode, MSize, MemoryError};

/// A raw record stored in memory, consisting of its length and data.
pub struct RawRecord<E>
where
    E: Encode,
{
    length: MSize,
    data: E,
}

impl<E> RawRecord<E>
where
    E: Encode,
{
    /// Creates a new raw record from the given data.
    pub fn new(data: E) -> Self {
        let length = data.size();
        Self { length, data }
    }
}

impl<E> Encode for RawRecord<E>
where
    E: Encode,
{
    const SIZE: crate::memory::DataSize = crate::memory::DataSize::Variable;

    fn size(&self) -> MSize {
        super::RECORD_LEN_SIZE + self.length // 2 bytes for length + data size
    }

    fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut encoded = Vec::with_capacity(self.size() as usize);
        encoded.extend_from_slice(&self.length.to_le_bytes());
        encoded.extend_from_slice(&self.data.encode());
        std::borrow::Cow::Owned(encoded)
    }

    fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
    where
        Self: Sized,
    {
        if data.len() < 2 {
            return Err(MemoryError::DecodeError(DecodeError::TooShort));
        }
        let length = u16::from_le_bytes([data[0], data[1]]) as MSize;
        if data.len() < 2 + length as usize {
            return Err(MemoryError::DecodeError(DecodeError::TooShort));
        }
        let data_slice = &data[2..2 + length as usize];
        let data_cow = std::borrow::Cow::Borrowed(data_slice);
        let data_decoded = E::decode(data_cow)?;
        Ok(Self {
            length,
            data: data_decoded,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_raw_record_encode_decode() {
        let record = TestRecord { a: 42, b: 65535 };
        let raw_record = RawRecord::new(record);
        let encoded = raw_record.encode();
        let decoded = RawRecord::<TestRecord>::decode(encoded).unwrap();
        assert_eq!(raw_record.length, decoded.length);
        assert_eq!(raw_record.data, decoded.data);
    }

    #[derive(Debug, PartialEq)]
    struct TestRecord {
        a: u8,
        b: u16,
    }

    impl Encode for TestRecord {
        const SIZE: crate::memory::DataSize = crate::memory::DataSize::Fixed(3);

        fn size(&self) -> MSize {
            3
        }

        fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
            std::borrow::Cow::Owned(vec![self.a, (self.b & 0xFF) as u8, (self.b >> 8) as u8])
        }

        fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
        where
            Self: Sized,
        {
            if data.len() != 3 {
                return Err(MemoryError::DecodeError(DecodeError::TooShort));
            }
            let a = data[0];
            let b = u16::from_le_bytes([data[1], data[2]]);
            Ok(Self { a, b })
        }
    }
}
