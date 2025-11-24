use crate::dbms::table::{ColumnDef, ForeignKeyDef, TableRecord, TableSchema};
use crate::dbms::types::{DataTypeKind, Text, Uint32};
use crate::memory::{DataSize, Encode};

/// A simple user struct for testing purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uint32,
    pub name: Text,
}

pub struct UserRecord {
    pub id: Option<Uint32>,
    pub name: Option<Text>,
}

impl TableRecord for UserRecord {
    type Schema = User;

    fn from_values(values: &[(ColumnDef, crate::dbms::value::Value)]) -> Self {
        let mut id = None;
        let mut name = None;

        for (col_def, value) in values {
            match col_def.name {
                "id" => {
                    if let crate::dbms::value::Value::Uint32(v) = value {
                        id = Some(*v);
                    }
                }
                "name" => {
                    if let crate::dbms::value::Value::Text(v) = value {
                        name = Some(v.clone());
                    }
                }
                _ => {}
            }
        }

        UserRecord { id, name }
    }

    fn to_values(&self) -> Vec<crate::dbms::value::Value> {
        let mut values = Vec::new();

        if let Some(id) = self.id {
            values.push(crate::dbms::value::Value::Uint32(id));
        } else {
            values.push(crate::dbms::value::Value::Null);
        }

        if let Some(name) = &self.name {
            values.push(crate::dbms::value::Value::Text(name.clone()));
        } else {
            values.push(crate::dbms::value::Value::Null);
        }

        values
    }
}

impl TableSchema for User {
    type Record = UserRecord;

    fn table_name() -> &'static str {
        "users"
    }

    fn columns() -> &'static [ColumnDef] {
        &[
            ColumnDef {
                name: "id",
                data_type: DataTypeKind::Uint32,
                nullable: false,
                primary_key: true,
                foreign_keys: None,
            },
            ColumnDef {
                name: "name",
                data_type: DataTypeKind::Text,
                nullable: false,
                primary_key: false,
                foreign_keys: None,
            },
        ]
    }

    fn primary_key() -> &'static str {
        "id"
    }

    fn foreign_keys() -> &'static [ForeignKeyDef] {
        &[]
    }
}

impl Encode for User {
    const SIZE: DataSize = DataSize::Variable;

    fn size(&self) -> crate::memory::MSize {
        self.id.size() + self.name.size()
    }

    fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut buffer = Vec::with_capacity(self.size() as usize);
        buffer.extend_from_slice(&self.id.encode());
        buffer.extend_from_slice(&self.name.encode());
        std::borrow::Cow::Owned(buffer)
    }

    fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
    where
        Self: Sized,
    {
        let id = Uint32::decode(std::borrow::Cow::Borrowed(&data[0..]))?;
        let offset = id.size() as usize;
        let name = Text::decode(std::borrow::Cow::Borrowed(&data[offset..]))?;

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
            id: 42u32.into(),
            name: "Alice".to_string().into(),
        };
        let encoded = user.encode();
        let decoded = User::decode(encoded).unwrap();
        assert_eq!(user, decoded);
    }

    #[test]
    fn test_should_have_fingerprint() {
        let fingerprint = User::fingerprint();
        assert_ne!(fingerprint, 0);
    }
}
