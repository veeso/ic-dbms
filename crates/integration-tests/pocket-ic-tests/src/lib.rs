mod actor;
mod pocket_ic;
pub mod table;
mod wasm;

use candid::{CandidType, Principal};
use serde::de::DeserializeOwned;

pub use self::pocket_ic::PocketIcTestEnv;

pub trait TestEnv {
    fn query<R>(
        &self,
        canister: Principal,
        caller: Principal,
        method: &str,
        payload: Vec<u8>,
    ) -> impl Future<Output = anyhow::Result<R>>
    where
        R: DeserializeOwned + CandidType;

    fn update<R>(
        &self,
        canister: Principal,
        caller: Principal,
        method: &str,
        payload: Vec<u8>,
    ) -> impl Future<Output = anyhow::Result<R>>
    where
        R: DeserializeOwned + CandidType;

    /// Admin principal id
    fn admin(&self) -> Principal;

    /// Bob principal id
    fn bob(&self) -> Principal;

    /// Alice principal id
    fn alice(&self) -> Principal;

    /// DBMS canister id
    fn dbms_canister(&self) -> Principal;
}
