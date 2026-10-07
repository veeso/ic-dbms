use ic_agent::Agent;
use ic_agent::identity::BasicIdentity;
use ic_dbms_api::prelude::Permission;
use ic_dbms_client::prelude::{Client, IcDbmsPocketIcClient};
use pocket_ic_harness::PocketIcTestEnv;

use crate::{TestCanister, TestEnvExt, admin};

/// Builds an agent on the live endpoint. With `grant_admin`, the agent's
/// principal is granted `Permission::Admin` on the DBMS canister by `admin()`.
pub async fn init_new_agent(ctx: &PocketIcTestEnv<TestCanister>, grant_admin: bool) -> Agent {
    let endpoint = ctx.endpoint().expect("context must be in live mode");

    let agent = Agent::builder()
        .with_url(endpoint)
        .with_identity(BasicIdentity::from_raw_key(&[1; 32]))
        .build()
        .expect("Failed to create agent");

    agent
        .fetch_root_key()
        .await
        .expect("Failed to fetch root key");

    if grant_admin {
        let canister_client = IcDbmsPocketIcClient::new(ctx.dbms_canister(), admin(), &ctx.pic);
        let agent_principal = agent
            .get_principal()
            .expect("failed to get agent's principal");
        canister_client
            .acl_grant(agent_principal, Permission::Admin)
            .await
            .expect("failed to call canister")
            .expect("failed to grant admin");
    }

    agent
}
