mod trap;

use candid::Principal;

pub use self::trap::trap;

/// Returns the caller's principal.
pub fn caller() -> Principal {
    #[cfg(target_family = "wasm")]
    {
        ic_cdk::api::msg_caller()
    }
    #[cfg(not(target_family = "wasm"))]
    {
        // dummy principal for non-wasm targets (e.g., during unit tests)
        Principal::from_text("ghsi2-tqaaa-aaaan-aaaca-cai").expect("it should be valid")
    }
}
