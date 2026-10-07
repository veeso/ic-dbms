use syn::Ident;

const ATTRIBUTE_TABLES: &str = "tables";
const RESERVED_TABLE_PREFIX: &str = "ic_dbms_";

pub struct CanisterMetadata {
    pub tables: Vec<TableMetadata>,
}

pub struct TableMetadata {
    pub name: String,
    pub table: Ident,
    pub record: Ident,
    pub insert: Ident,
    pub update: Ident,
}

/// Collects canister metadata from the given attributes.
pub fn collect_canister_metadata(attrs: &[syn::Attribute]) -> syn::Result<CanisterMetadata> {
    let mut tables = Vec::new();
    let mut names = vec![];

    for attr in attrs {
        if attr.path().is_ident(ATTRIBUTE_TABLES) {
            attr.parse_nested_meta(|meta| {
                let ident = meta
                    .path
                    .get_ident()
                    .cloned()
                    .ok_or_else(|| meta.error("expected identifier"))?;
                let value: syn::LitStr = meta.value()?.parse()?;
                let value = value.value();

                names.push((ident, value));

                Ok(())
            })
            .expect("invalid syntax in #[tables]");
        }
    }

    for (ident, name) in names {
        if name.starts_with(RESERVED_TABLE_PREFIX) {
            return Err(syn::Error::new(
                ident.span(),
                format!(
                    "table name `{name}` is reserved: names starting with `{RESERVED_TABLE_PREFIX}` belong to ic-dbms"
                ),
            ));
        }
        tables.push(collect_table_metadata(ident, name)?);
    }

    Ok(CanisterMetadata { tables })
}

/// Collects metadata for a database table from its name.
fn collect_table_metadata(table: Ident, name: String) -> syn::Result<TableMetadata> {
    let record_ident = Ident::new(&format!("{table}Record"), table.span());
    let insert_ident = Ident::new(&format!("{table}InsertRequest"), table.span());
    let update_ident = Ident::new(&format!("{table}UpdateRequest"), table.span());

    Ok(TableMetadata {
        table: table.clone(),
        record: record_ident,
        insert: insert_ident,
        update: update_ident,
        name,
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_should_reject_reserved_table_name() {
        let attr: syn::Attribute = syn::parse_quote!(#[tables(Acl = "ic_dbms_acl")]);
        let err = match collect_canister_metadata(&[attr]) {
            Ok(_) => panic!("reserved name must be rejected"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("reserved"), "{err}");
    }

    #[test]
    fn test_should_accept_regular_table_names() {
        let attr: syn::Attribute = syn::parse_quote!(#[tables(User = "users", Post = "posts")]);
        let metadata = collect_canister_metadata(&[attr]).expect("valid tables");
        assert_eq!(metadata.tables.len(), 2);
        assert_eq!(metadata.tables[0].name, "users");
        assert_eq!(metadata.tables[1].record.to_string(), "PostRecord");
    }
}
