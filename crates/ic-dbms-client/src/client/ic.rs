use candid::Principal;

/// Client to interact with an IC DBMS Canister.
#[derive(Clone, Debug)]
pub struct IcDbmsCanisterClient {
    principal: Principal,
}

impl From<Principal> for IcDbmsCanisterClient {
    fn from(principal: Principal) -> Self {
        Self { principal }
    }
}

impl IcDbmsCanisterClient {
    /// Creates a new IC DBMS Canister client from the given [`Principal`].
    pub fn new(principal: Principal) -> Self {
        Self::from(principal)
    }
}
