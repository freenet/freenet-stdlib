//! Interface and related utilities for interaction with the compiled WASM contracts.
//! Contracts have an isomorphic interface which partially maps to this interface,
//! allowing interaction between the runtime and the contracts themselves.
//!
//! This abstraction layer shouldn't leak beyond the contract handler.

pub(crate) const CONTRACT_KEY_SIZE: usize = 32;

mod error;
mod state;
mod key;
mod code;
mod update;
mod contract;
mod wrapped;
mod trait_def;
pub(crate) mod wasm_interface;
pub mod encoding;

#[cfg(all(test, any(unix, windows)))]
mod tests;

// Re-export all public types
pub use error::ContractError;
pub use state::{State, StateDelta, StateSummary};
pub use key::{ContractInstanceId, ContractKey};
pub use code::ContractCode;
pub use update::{
    UpdateData, UpdateModification, RelatedContracts, RelatedContract, RelatedMode, ValidateResult,
};
pub use contract::Contract;
pub use wrapped::{WrappedState, WrappedContract};
pub use trait_def::ContractInterface;
