use crate::prelude::{ColumnDef, Value, ValuesSource};

/// Helper function which takes a list of `(ValuesSource, Value)` tuples, take only those with
/// [`ValuesSource::Foreign`] matching the provided table and column names, and returns a vector of
/// the corresponding `Value`s. with the [`ValuesSource`] set to [`ValuesSource::This`].
pub fn self_reference_values(
    values: &[(ValuesSource, Vec<(ColumnDef, Value)>)],
    table: &'static str,
    local_column: &'static str,
) -> Vec<(ValuesSource, Vec<(ColumnDef, Value)>)> {
    values
        .iter()
        .filter(|(source, _)| matches!(source, ValuesSource::Foreign { table: t, column } if *t == table && *column == local_column))
        .map(|(_, value)| (ValuesSource::This, value.clone())
    )
    .collect()
}

#[cfg(test)]
mod tests {

    use ic_dbms_api::prelude::TableSchema;

    use super::*;
    use crate::tests::User;

    #[test]
    fn test_self_reference_values() {
        let col = User::columns()[0]; // id column

        let values = vec![(
            ValuesSource::Foreign {
                table: "users".to_string(),
                column: "id".to_string(),
            },
            vec![(col, ic_dbms_api::prelude::Value::Uint64(42.into()))],
        )];

        let result = self_reference_values(&values, "users", "id");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, ValuesSource::This);
        assert_eq!(result[0].1, values[0].1);
    }
}
