mod agent;
mod client;
pub mod table;

use candid::{Encode, Principal};
use ic_dbms_api::prelude::{IcDbmsCanisterArgs, IcDbmsCanisterInitArgs};
use pocket_ic_harness::{Canister, PocketIcTestEnv};
pub use pocket_ic_harness::{admin, alice, bob};

pub use self::agent::init_new_agent;
pub use self::client::PocketIcClient;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TestCanister {
    DbmsCanister,
    DbmsCanisterClientIntegration,
}

impl Canister for TestCanister {
    fn as_path(&self) -> &'static std::path::Path {
        match self {
            TestCanister::DbmsCanister => std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../.artifact/example.wasm.gz"
            )),
            TestCanister::DbmsCanisterClientIntegration => std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../.artifact/dbms_canister_client_integration.wasm.gz"
            )),
        }
    }

    fn all_canisters() -> &'static [Self] {
        &[Self::DbmsCanister, Self::DbmsCanisterClientIntegration]
    }

    fn init_arg(&self, env: &PocketIcTestEnv<Self>) -> Vec<u8> {
        match self {
            TestCanister::DbmsCanister => {
                Encode!(&IcDbmsCanisterArgs::Init(IcDbmsCanisterInitArgs {
                    allowed_principals: Some(vec![
                        admin(),
                        env.canister_id(&TestCanister::DbmsCanisterClientIntegration)
                    ]),
                }))
                .expect("failed to encode dbms canister init args")
            }
            TestCanister::DbmsCanisterClientIntegration => {
                let dbms_canister = env.canister_id(&TestCanister::DbmsCanister);
                Encode!(&dbms_canister).expect("Failed to encode init arg")
            }
        }
    }
}

pub trait TestEnvExt {
    fn dbms_canister(&self) -> Principal;
    fn dbms_canister_client_integration(&self) -> Principal;
}

impl TestEnvExt for PocketIcTestEnv<TestCanister> {
    fn dbms_canister(&self) -> Principal {
        self.canister_id(&TestCanister::DbmsCanister)
    }

    fn dbms_canister_client_integration(&self) -> Principal {
        self.canister_id(&TestCanister::DbmsCanisterClientIntegration)
    }
}
