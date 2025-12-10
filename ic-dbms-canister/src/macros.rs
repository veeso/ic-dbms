/// Macro to define a DBMS canister with specified entities and tables.
///
/// It is a convenience macro that generates a canister struct annotated with the `DbmsCanister` derive macro,
/// specifying the entities and tables to be managed by the canister.
///
/// The syntax for it is:
///
/// ```rust,ignore
/// ic_dbms_canister! {
///     Entity1 => table1,
///     Entity2 => table2,
/// }
/// ```
#[macro_export]
macro_rules! ic_dbms_canister {
    ( $( $entity:ident => $table:ident ),* $(,)? ) => {
        use $crate::prelude::DbmsCanister;
        #[derive(DbmsCanister)]
        #[allow(dead_code)]
        #[entities( $( $entity ),* )]
        #[tables( $( $table ),* )]
        struct IcDbmsCanisterGenerator;
    };
}
