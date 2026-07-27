//! A node client API. Intended to be used from applications (web or otherwise) using the
//! node capabilities to execute contract, delegate, etc. instructions and communicating
//! over the network.
//!
//! Communication, independent of the transport, revolves around the [`ClientRequest`]
//! and [`HostResponse`] types.
//!
//! Currently the clients available are:
//! - `websocket`:
//!   - `regular` (native): Using TCP transport directly, for native applications programmed in Rust.
//!   - `browser` (wasm): Via wasm-bindgen (and by extension web-sys).
//!     (In order to use this client from JS/Typescript refer to the Typescript std lib).
mod client_events;

#[cfg(all(any(unix, windows), feature = "net"))]
mod regular;
#[cfg(all(any(unix, windows), feature = "net"))]
pub use regular::*;

#[cfg(all(target_family = "wasm", feature = "net"))]
mod browser;
#[cfg(all(target_family = "wasm", feature = "net"))]
pub use browser::*;

#[cfg(feature = "net")]
pub mod streaming;

pub use client_events::*;

#[cfg(feature = "net")]
type HostResult = Result<HostResponse, ClientError>;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Deserialization(#[from] bincode::Error),
    #[error("channel closed")]
    ChannelClosed,
    #[cfg(all(any(unix, windows), feature = "net"))]
    #[error(transparent)]
    ConnectionError(#[from] tokio_tungstenite::tungstenite::Error),
    #[cfg(all(target_family = "wasm", feature = "net"))]
    #[error("request error: {0}")]
    ConnectionError(serde_json::Value),
    #[error("connection closed")]
    ConnectionClosed,
    #[error("unhandled error: {0}")]
    OtherError(Box<dyn std::error::Error + Send + Sync>),
}

pub trait TryFromFbs<T>: Sized {
    fn try_decode_fbs(value: T) -> Result<Self, WsApiError>;
}

/// Read a fixed-size byte field out of a verified flatbuffer, rejecting a wrong
/// length instead of panicking.
///
/// **Every** fixed-size wire field must go through this. The flatbuffers
/// verifier checks that a `(required)` vector is PRESENT; it does NOT check its
/// LENGTH (`Verifiable for Vector<T>` runs `verify_vector_range` and nothing
/// else). So `flatbuffers::root` happily accepts an 8-byte field declared
/// `(required)`, and any decoder that then assumes 32 bytes — via
/// `try_into().unwrap()`, `<[u8; N]>::try_from(..).unwrap()`, or
/// `copy_from_slice` — panics on it. Nothing catches unwind on the decode path
/// and `panic = "abort"` is not set, so that unwinds and kills the client's
/// connection task: a remote, wire-reachable panic.
///
/// `field` is the schema path of the offending field (e.g.
/// `"ContractKey.instance"`), so the error names the exact thing the client got
/// wrong rather than saying "invalid data".
pub(crate) fn fixed_size_field<const N: usize>(
    field: &str,
    data: &[u8],
) -> Result<[u8; N], WsApiError> {
    data.try_into().map_err(|_| {
        WsApiError::deserialization(format!(
            "{field} must be exactly {N} bytes; got {} bytes. The flatbuffers verifier only \
             checks that this required field is present, not that it is the right length, so a \
             wrong-length value reaches the decoder and must be rejected here.",
            data.len()
        ))
    })
}

/// Error text for an unknown flatbuffers union discriminant.
///
/// Every generated union verifier ends in `_ => Ok(())`, so a discriminant the
/// schema does not define — including `NONE` (0), which several TypeScript SDK
/// constructors take as their default argument — passes verification and
/// reaches the decoder's match. Matching such a value with `unreachable!()`
/// turns one crafted (or merely mistaken) request into a panic that downs the
/// connection handler, so every union match returns this instead.
pub(crate) fn unknown_union_discriminant(union: &str, discriminant: u8) -> WsApiError {
    WsApiError::deserialization(format!(
        "unknown {union} discriminant: {discriminant}. The flatbuffers verifier accepts any \
         discriminant it does not recognize, so this is a client error, not an impossible state."
    ))
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum WsApiError {
    #[error("Unsupported contract version")]
    UnsupportedContractVersion,
    #[error("Failed unpacking contract container")]
    UnpackingContractContainerError(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed decoding message from client request: {cause}")]
    DeserError { cause: String },
}

impl WsApiError {
    pub fn deserialization(cause: String) -> Self {
        Self::DeserError { cause }
    }

    pub fn into_fbs_bytes(self) -> Vec<u8> {
        use crate::generated::host_response::{
            finish_host_response_buffer, Error, ErrorArgs, HostResponse, HostResponseArgs,
            HostResponseType,
        };
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let as_msg = format!("{self}");
        let msg_offset = builder.create_string(&as_msg);
        let err_offset = Error::create(
            &mut builder,
            &ErrorArgs {
                msg: Some(msg_offset),
            },
        );
        let res = HostResponse::create(
            &mut builder,
            &HostResponseArgs {
                response_type: HostResponseType::Error,
                response: Some(err_offset.as_union_value()),
            },
        );
        finish_host_response_buffer(&mut builder, res);
        builder.finished_data().to_vec()
    }
}
