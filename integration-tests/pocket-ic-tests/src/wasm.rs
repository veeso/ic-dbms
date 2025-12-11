use std::path::Path;

pub enum Canister {
    DbmsCanister,
}

impl Canister {
    pub fn as_path(&self) -> &'static Path {
        match self {
            Canister::DbmsCanister => Path::new("../../.artifact/example.wasm.gz"),
        }
    }
}
