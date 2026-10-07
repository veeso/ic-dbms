mod metadata;

use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::DeriveInput;

use self::metadata::TableMetadata;

pub fn dbms_canister(input: DeriveInput) -> syn::Result<TokenStream2> {
    let metadata = self::metadata::collect_canister_metadata(&input.attrs)?;
    let struct_ident = &input.ident;

    let init_fn = impl_init(&metadata.tables, struct_ident);
    let inspect_fn = impl_inspect();
    let pre_upgrade_fn = impl_pre_upgrade();
    let acl_api = impl_acl_api(struct_ident);
    let transaction_api = impl_transaction_api(struct_ident);
    let tables_api = impl_tables_api(&metadata.tables, struct_ident);
    let select_raw_api = impl_select_raw_api(struct_ident);
    let migration_api = impl_migration_api(struct_ident);
    let sql_api = impl_sql_api(struct_ident, cfg!(feature = "sql"));

    Ok(quote::quote! {
        #init_fn
        #inspect_fn
        #pre_upgrade_fn
        #acl_api
        #transaction_api
        #tables_api
        #select_raw_api
        #migration_api
        #sql_api
    })
}

fn impl_init(tables: &[TableMetadata], struct_ident: &syn::Ident) -> TokenStream2 {
    let mut init_tables = vec![];
    for table in tables {
        let table_name = &table.table;
        let table_str = table_name.to_string();
        init_tables.push(quote::quote! {
            ::ic_dbms_canister::prelude::DBMS_CONTEXT.with(|ctx| {
                if let Err(err) = ctx.register_table::<#table_name>() {
                    ::ic_cdk::trap(&format!(
                        "Failed to register table {} during init: {}",
                        #table_str, err
                    ));
                }
            });
        });
    }

    quote::quote! {
        #[::ic_cdk::init]
        fn init(args: ::ic_dbms_api::prelude::IcDbmsCanisterArgs) {
            let args = args.unwrap_init();
            let principals = match args.allowed_principals {
                Some(p) if !p.is_empty() => p,
                _ => vec![::ic_cdk::api::msg_caller()],
            };
            #(#init_tables)*
            if let Err(err) = ::ic_dbms_canister::api::init_acl(principals, #struct_ident) {
                ::ic_cdk::trap(&format!("Failed to bootstrap ACL during init: {}", err));
            }
        }
    }
}

fn impl_inspect() -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::inspect_message]
        fn inspect() {
            ::ic_dbms_canister::api::inspect()
        }
    }
}

fn impl_pre_upgrade() -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::pre_upgrade]
        fn pre_upgrade() {
            ::ic_dbms_canister::api::pre_upgrade()
        }
    }
}

fn impl_tables_api(tables: &[TableMetadata], struct_ident: &syn::Ident) -> TokenStream2 {
    let table_apis: Vec<_> = tables
        .iter()
        .map(|table| impl_table_api(table, struct_ident))
        .collect();

    quote::quote! {
        #(#table_apis)*
    }
}

fn impl_transaction_api(struct_ident: &syn::Ident) -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::update]
        fn begin_transaction() -> ::ic_dbms_api::prelude::IcDbmsResult<::ic_dbms_api::prelude::TransactionId> {
            ::ic_dbms_canister::api::begin_transaction(#struct_ident)
        }

        #[::ic_cdk::update]
        fn commit(transaction_id: ::ic_dbms_api::prelude::TransactionId) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::commit(transaction_id, #struct_ident)
        }

        #[::ic_cdk::update]
        fn rollback(transaction_id: ::ic_dbms_api::prelude::TransactionId) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::rollback(transaction_id, #struct_ident)
        }
    }
}

fn impl_acl_api(struct_ident: &syn::Ident) -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::update]
        fn acl_grant(
            grant: ::ic_dbms_api::prelude::AclGrant,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::acl_grant(grant, #struct_ident)
        }

        #[::ic_cdk::update]
        fn acl_revoke(
            grant: ::ic_dbms_api::prelude::AclGrant,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::acl_revoke(grant, #struct_ident)
        }

        #[::ic_cdk::query]
        fn acl_list() -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<::ic_dbms_api::prelude::AclGrant>> {
            ::ic_dbms_canister::api::acl_list(#struct_ident)
        }

        #[::ic_cdk::query]
        fn my_permissions() -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<::ic_dbms_api::prelude::AclGrant>> {
            ::ic_dbms_canister::api::my_permissions(#struct_ident)
        }
    }
}

fn impl_select_raw_api(struct_ident: &syn::Ident) -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::query]
        fn select(
            table: String,
            query: ::ic_dbms_api::prelude::Query,
            transaction_id: Option<::ic_dbms_api::prelude::TransactionId>,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<Vec<(::ic_dbms_api::prelude::JoinColumnDef, ::ic_dbms_api::prelude::Value)>>> {
            if query.has_joins() {
                ::ic_dbms_canister::api::select_join(&table, query, transaction_id, #struct_ident)
            } else {
                ::ic_dbms_canister::api::select_raw(&table, query, transaction_id, #struct_ident)
                    .map(|rows| {
                        rows.into_iter()
                            .map(|row| {
                                row.into_iter()
                                    .map(|(col, val)| (::ic_dbms_api::prelude::JoinColumnDef::from(col), val))
                                    .collect()
                            })
                            .collect()
                    })
            }
        }
    }
}

fn impl_migration_api(struct_ident: &syn::Ident) -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::query]
        fn has_drift() -> ::ic_dbms_api::prelude::IcDbmsResult<bool> {
            ::ic_dbms_canister::api::has_drift(#struct_ident)
        }

        #[::ic_cdk::query]
        fn pending_migrations() -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<::ic_dbms_api::prelude::MigrationOp>> {
            ::ic_dbms_canister::api::pending_migrations(#struct_ident)
        }

        #[::ic_cdk::update]
        fn migrate(policy: ::ic_dbms_api::prelude::MigrationPolicy) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::migrate(policy, #struct_ident)
        }
    }
}

/// Emits the `sql` update endpoint and the `sql_query` query endpoint, or
/// nothing when `enabled` is `false` (the `sql` feature is off).
fn impl_sql_api(struct_ident: &syn::Ident, enabled: bool) -> TokenStream2 {
    if !enabled {
        return TokenStream2::new();
    }
    quote::quote! {
        #[::ic_cdk::update]
        fn sql(
            query: String,
            params: Vec<::ic_dbms_api::prelude::Value>,
            transaction_id: Option<::ic_dbms_api::prelude::TransactionId>,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<::ic_dbms_api::prelude::SqlResult> {
            ::ic_dbms_canister::api::sql(&query, &params, transaction_id, #struct_ident)
        }

        #[::ic_cdk::query]
        fn sql_query(
            query: String,
            params: Vec<::ic_dbms_api::prelude::Value>,
            transaction_id: Option<::ic_dbms_api::prelude::TransactionId>,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<::ic_dbms_api::prelude::SqlResult> {
            ::ic_dbms_canister::api::sql_query(&query, &params, transaction_id, #struct_ident)
        }
    }
}

fn impl_table_api(table: &TableMetadata, struct_ident: &syn::Ident) -> TokenStream2 {
    let table_name = &table.name;
    let entity = &table.table;
    let record = &table.record;
    let insert = &table.insert;
    let update = &table.update;
    let select_fn_name = format_ident!("select_{}", table_name);
    let aggregate_fn_name = format_ident!("aggregate_{}", table_name);
    let insert_fn_name = format_ident!("insert_{}", table_name);
    let update_fn_name = format_ident!("update_{}", table_name);
    let delete_fn_name = format_ident!("delete_{}", table_name);

    quote::quote! {
        #[::ic_cdk::query]
        fn #select_fn_name(query: ::ic_dbms_api::prelude::Query, transaction_id: Option<::ic_dbms_api::prelude::TransactionId>) -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<#record>> {
            ::ic_dbms_canister::api::select::<#entity, #struct_ident>(query, transaction_id, #struct_ident)
        }

        #[::ic_cdk::query]
        fn #aggregate_fn_name(
            query: ::ic_dbms_api::prelude::Query,
            aggregates: Vec<::ic_dbms_api::prelude::AggregateFunction>,
            transaction_id: Option<::ic_dbms_api::prelude::TransactionId>,
        ) -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<::ic_dbms_api::prelude::AggregatedRow>> {
            ::ic_dbms_canister::api::aggregate::<#entity, #struct_ident>(query, aggregates, transaction_id, #struct_ident)
        }

        #[::ic_cdk::update]
        fn #insert_fn_name(record: #insert, transaction_id: Option<::ic_dbms_api::prelude::TransactionId>) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::insert::<#entity, #struct_ident>(record, transaction_id, #struct_ident)
        }

        #[::ic_cdk::update]
        fn #update_fn_name(patch: #update, transaction_id: Option<::ic_dbms_api::prelude::TransactionId>) -> ::ic_dbms_api::prelude::IcDbmsResult<u64> {
            ::ic_dbms_canister::api::update::<#entity, #struct_ident>(patch, transaction_id, #struct_ident)
        }

        #[::ic_cdk::update]
        fn #delete_fn_name(delete_behavior: ::ic_dbms_api::prelude::DeleteBehavior, filter: Option<::ic_dbms_api::prelude::Filter>, transaction_id: Option<::ic_dbms_api::prelude::TransactionId>) -> ::ic_dbms_api::prelude::IcDbmsResult<u64> {
            ::ic_dbms_canister::api::delete::<#entity, #struct_ident>(delete_behavior, filter, transaction_id, #struct_ident)
        }
    }
}

#[cfg(test)]
mod tests {

    use syn::parse_quote;

    use super::*;

    /// Returns `(name, "update" | "query")` for every endpoint in `tokens`.
    fn endpoints(tokens: TokenStream2) -> Vec<(String, String)> {
        let file: syn::File = syn::parse2(tokens).expect("macro output must be valid items");
        file.items
            .into_iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => {
                    let kind = function.attrs.iter().find_map(|attr| {
                        let name = attr.path().segments.last()?.ident.to_string();
                        matches!(name.as_str(), "update" | "query").then_some(name)
                    })?;
                    Some((function.sig.ident.to_string(), kind))
                }
                _ => None,
            })
            .collect()
    }

    fn schema_ident() -> syn::Ident {
        format_ident!("Schema")
    }

    #[test]
    fn test_should_emit_sql_endpoints_when_enabled() {
        assert_eq!(
            endpoints(impl_sql_api(&schema_ident(), true)),
            vec![
                ("sql".to_string(), "update".to_string()),
                ("sql_query".to_string(), "query".to_string()),
            ]
        );
    }

    #[test]
    fn test_should_not_emit_sql_endpoints_when_disabled() {
        assert!(impl_sql_api(&schema_ident(), false).is_empty());
    }

    #[test]
    fn test_should_follow_sql_feature_in_derive() {
        let input: DeriveInput = parse_quote! {
            #[tables(User = "users")]
            struct Schema;
        };
        let names: Vec<String> = endpoints(dbms_canister(input).expect("derive"))
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let has_sql = names
            .iter()
            .any(|name| name == "sql" || name == "sql_query");
        assert_eq!(has_sql, cfg!(feature = "sql"));
        assert!(names.iter().any(|name| name == "select_users"));
    }
}
