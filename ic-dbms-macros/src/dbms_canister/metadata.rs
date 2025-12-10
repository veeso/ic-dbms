use syn::Ident;

const ATTRIBUTE_TABLES: &str = "tables";
const ATTRIBUTE_ENTITIES: &str = "entities";
const ATTRIBUTE_REFERENCED_TABLES: &str = "referenced_tables";

pub struct CanisterMetadata {
    pub tables: Vec<TableMetadata>,
    pub referenced_tables: Ident,
}

pub struct TableMetadata {
    pub name: Ident,
    pub table: Ident,
    pub record: Ident,
    pub insert: Ident,
    pub update: Ident,
}

/// Collects canister metadata from the given attributes.
pub fn collect_canister_metadata(attrs: &[syn::Attribute]) -> syn::Result<CanisterMetadata> {
    let mut tables = Vec::new();

    let mut names = vec![];
    let mut entities = vec![];
    let mut referenced_tables = None;

    for attr in attrs {
        if attr.path().is_ident(ATTRIBUTE_ENTITIES) {
            attr.parse_nested_meta(|meta| {
                let ident = meta
                    .path
                    .get_ident()
                    .cloned()
                    .ok_or_else(|| meta.error("expected identifier"))?;

                entities.push(ident.clone());

                Ok(())
            })
            .expect("invalid syntax in #[entities]");
        } else if attr.path().is_ident(ATTRIBUTE_TABLES) {
            attr.parse_nested_meta(|meta| {
                // get literal string
                let ident = meta
                    .path
                    .get_ident()
                    .cloned()
                    .ok_or_else(|| meta.error("expected identifier"))?;

                names.push(ident);

                Ok(())
            })
            .expect("invalid syntax in #[tables]");
        } else if attr.path().is_ident(ATTRIBUTE_REFERENCED_TABLES) {
            attr.parse_nested_meta(|meta| {
                let ident = meta
                    .path
                    .get_ident()
                    .cloned()
                    .ok_or_else(|| meta.error("expected identifier"))?;

                referenced_tables = Some(ident);

                Ok(())
            })
            .expect("invalid syntax in #[referenced_tables]");
        }
    }

    if entities.len() != names.len() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "mismatched number of tables and names: {} tables, {} names",
                entities.len(),
                names.len()
            ),
        ));
    }

    if referenced_tables.is_none() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "missing #[referenced_tables] attribute".to_string(),
        ));
    }

    for (ident, name) in entities.into_iter().zip(names.into_iter()) {
        tables.push(collect_table_metadata(ident, name)?);
    }

    Ok(CanisterMetadata {
        tables,
        referenced_tables: referenced_tables.unwrap(),
    })
}

/// Collects metadata for a database table from its name.
fn collect_table_metadata(table: Ident, name: Ident) -> syn::Result<TableMetadata> {
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
