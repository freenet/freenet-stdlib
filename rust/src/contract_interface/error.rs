//! Error types for contract interface operations.

use serde::{Deserialize, Serialize};

/// Type of errors during interaction with a contract.
///
/// Marked `#[non_exhaustive]` so future error variants can be added without
/// a source-level break. Downstream `match` sites must include a wildcard arm.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum ContractError {
    #[error("de/serialization error: {0}")]
    Deser(String),
    #[error("invalid contract update")]
    InvalidUpdate,
    #[error("invalid contract update, reason: {reason}")]
    InvalidUpdateWithInfo { reason: String },
    #[error("trying to read an invalid state")]
    InvalidState,
    #[error("trying to read an invalid delta")]
    InvalidDelta,
    #[error("{0}")]
    Other(String),
}
