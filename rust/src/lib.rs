//! Standard library provided by the Freenet project to be able to write Locutus-compatible contracts.
mod code_hash;
mod composers;
mod contract_interface;
mod delegate_interface;
pub(crate) mod global;
pub mod memory;
mod parameters;
mod versioning;

#[allow(dead_code, unused_imports, clippy::all)]
pub(crate) mod client_request_generated;
#[allow(dead_code, unused_imports, clippy::all)]
pub(crate) mod common_generated;
#[allow(dead_code, unused_imports, clippy::all)]
pub(crate) mod host_response_generated;

pub mod client_api;
#[cfg(all(feature = "log", target_family = "wasm"))]
pub mod log;
pub mod rand;
#[cfg(all(feature = "time", target_family = "wasm"))]
pub mod time;

/// Locutus stdlib prelude.
pub mod prelude {
    pub use crate::code_hash::*;
    pub use crate::contract_interface::wasm_interface::*;
    pub use crate::contract_interface::*;
    pub use crate::delegate_interface::wasm_interface::*;
    pub use crate::delegate_interface::*;
    pub use crate::parameters::*;
    pub use crate::versioning::*;
    pub use freenet_macros::{contract, delegate};

    pub use bincode;
    pub use blake3;
    pub use serde_json;
    pub use tracing;
    pub use tracing_subscriber;
}
