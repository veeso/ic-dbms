//! Debugging expand results

use candid::CandidType;
use ic_dbms_api::prelude::{Nullable, Table, Text, Uint32, Uint64};
//use ic_dbms_canister::prelude::DbmsCanister;
use serde::Deserialize;

#[derive(Clone, Table, CandidType, Deserialize)]
#[table = "users"]
pub struct User {
    #[primary_key]
    id: Uint64,
    name: Text,
    email: Text,
    age: Nullable<Uint32>,
    nickname: Nullable<Text>,
    #[foreign_key(entity = "User", table = "users", column = "id")]
    father: Uint64,
}

/*
#[derive(Clone)]
struct User {
    id: Uint64,
    name: Text,
    email: Text,
    age: Nullable<Uint32>,
    nickname: Nullable<Text>,
    father: Uint64,
}
     */

//#[derive(DbmsCanister)]
//#[allow(dead_code)]
//#[entities(User)]
//#[tables(users)]
//struct Canister;

fn main() {
    println!("Hello, world!");
}
