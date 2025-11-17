use crate::memory::{DataSize, Encode, MSize};

/// A simple user struct for testing purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u32,
    pub name: String,
}

impl Encode for User {
    const SIZE: DataSize = DataSize::Variable;

    fn size(&self) -> crate::memory::MSize {
        std::mem::size_of::<u32>() as crate::memory::MSize
            + std::mem::size_of::<MSize>() as crate::memory::MSize
            + (self.name.len() as crate::memory::MSize)
    }

    fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut buffer = Vec::with_capacity(self.size() as usize);
        buffer.extend_from_slice(&self.id.to_le_bytes());
        let name_len = self.name.len() as MSize;
        buffer.extend_from_slice(&name_len.to_le_bytes());
        buffer.extend_from_slice(self.name.as_bytes());
        std::borrow::Cow::Owned(buffer)
    }

    fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
    where
        Self: Sized,
    {
        let id = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let name_len = MSize::from_le_bytes(
            data[4..4 + std::mem::size_of::<MSize>()]
                .try_into()
                .unwrap(),
        );
        let name = String::from_utf8(
            data[4 + std::mem::size_of::<MSize>()
                ..4 + std::mem::size_of::<MSize>() + name_len as usize]
                .to_vec(),
        )
        .unwrap();
        Ok(User { id, name })
    }
}

#[allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_encode_decode() {
        let user = User {
            id: 42,
            name: "Alice".to_string(),
        };
        let encoded = user.encode();
        let decoded = User::decode(encoded).unwrap();
        assert_eq!(user, decoded);
    }
}
