use candid::CandidType;
use ic_dbms_api::prelude::{Text, Uint32};
use ic_dbms_canister::prelude::{DbmsCanister, Table};
use serde::Deserialize;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    pub email: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
    pub content: Text,
    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub user: Uint32,
}

#[derive(DbmsCanister)]
#[tables(User = "users", Post = "posts")]
pub struct IcDbmsCanisterGenerator;

ic_cdk::export_candid!();
