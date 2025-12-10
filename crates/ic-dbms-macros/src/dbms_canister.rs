mod metadata;

use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::DeriveInput;

use self::metadata::TableMetadata;

pub fn dbms_canister(input: DeriveInput) -> syn::Result<TokenStream2> {
    let metadata = self::metadata::collect_canister_metadata(&input.attrs)?;

    let init_fn = impl_init(&metadata.tables);
    let acl_api = impl_acl_api();
    let tables_api = impl_tables_api(&metadata.tables);

    Ok(quote::quote! {
        #init_fn
        #acl_api
        #tables_api
    })
}

fn impl_init(tables: &[TableMetadata]) -> TokenStream2 {
    let mut init_tables = vec![];
    for table in tables {
        let table_name = &table.table;
        let table_str = table_name.to_string();
        init_tables.push(quote::quote! {
                ::ic_dbms_canister::prelude::SCHEMA_REGISTRY.with_borrow_mut(|registry| {
                if let Err(err) = registry.register_table::<#table_name>() {
                    ::ic_cdk::trap(&format!(
                        "Failed to register table {} during init: {}",
                        #table_str,
                        err
                    ));
                }
            })
        });
    }

    quote::quote! {
        #[::ic_cdk::init]
        fn init(args: ::ic_dbms_api::prelude::IcDbmsCanisterArgs) {
            let args = args.unwrap_init();
            ::ic_dbms_canister::prelude::ACL.with_borrow_mut(|acl| {
                for principal in args.allowed_principals {
                    if let Err(err) = acl.add_principal(principal) {
                        ::ic_cdk::trap(&format!(
                            "Failed to add principal to ACL during init: {}",
                            err
                        ));
                    }
                }
            });

            // init tables
            #(#init_tables)*
        }
    }
}

fn impl_acl_api() -> TokenStream2 {
    quote::quote! {
        #[::ic_cdk::update]
        fn acl_add_principal(principal: ::candid::Principal) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::acl_add_principal(principal)
        }

        #[::ic_cdk::update]
        fn acl_remove_principal(principal: ::candid::Principal) -> ::ic_dbms_api::prelude::IcDbmsResult<()> {
            ::ic_dbms_canister::api::acl_remove_principal(principal)
        }

        #[::ic_cdk::query]
        fn acl_allowed_principals() -> Vec<::candid::Principal> {
            ::ic_dbms_canister::api::acl_allowed_principals()
        }
    }
}

fn impl_tables_api(tables: &[TableMetadata]) -> TokenStream2 {
    let mut table_apis = vec![];
    for table in tables {
        table_apis.push(impl_table_api(table));
    }

    quote::quote! {
        #(#table_apis)*
    }
}

fn impl_table_api(table: &TableMetadata) -> TokenStream2 {
    let table_name = &table.name;
    let entity = &table.table;
    let record = &table.record;
    let select_fn_name = format_ident!("select_{}", table_name);
    let insert_fn_name = format_ident!("insert_{}", table_name);
    let update_fn_name = format_ident!("update_{}", table_name);
    let delete_fn_name = format_ident!("delete_{}", table_name);

    quote::quote! {
        #[::ic_cdk::query]
        fn #select_fn_name(query: ::ic_dbms_api::prelude::Query<#entity>, transaction_id: Option<::ic_dbms_api::prelude::TransactionId>) -> ::ic_dbms_api::prelude::IcDbmsResult<Vec<#record>> {
            ::ic_dbms_canister::api::select(query, transaction_id, todo!())
        }
    }
}
