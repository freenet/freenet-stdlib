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
    // The message goes over the wire to the client, so it carries what the
    // client can act on — the field, the expected length, the observed length —
    // and nothing else. The reason this check has to exist at all is stdlib's
    // business and lives in the doc comment above.
    data.try_into().map_err(|_| {
        WsApiError::deserialization(format!(
            "{field} must be exactly {N} bytes; got {} bytes",
            data.len()
        ))
    })
}

/// Error text for an unknown flatbuffers union discriminant.
///
/// Every generated union verifier ends in `_ => Ok(())`, so a discriminant the
/// schema does not define passes verification and reaches the decoder's match.
/// Matching such a value with `unreachable!()` turns one crafted request into a
/// panic that downs the connection handler, so every union match returns this
/// instead.
///
/// Be precise about which values actually get here, because the obvious guess
/// is wrong. A `NONE` (0) discriminant does NOT reach the decoder: both the
/// Rust and TypeScript builders elide a field equal to its default
/// (`FlatBufferBuilder::push_slot`), so `NONE` is written as ABSENT, and
/// `Verifier::visit_union` rejects the resulting present-value/absent-type pair
/// as an inconsistent union before any decoder runs. What reaches here is an
/// out-of-range discriminant, or a `NONE` written explicitly by an encoder that
/// forces defaults or is not flatc-generated. That is enough — it is still
/// wire-reachable — but "the TypeScript SDK defaults to NONE" is not the
/// mechanism, and a test that only checks NONE pins nothing.
pub(crate) fn unknown_union_discriminant(union: &str, discriminant: u8) -> WsApiError {
    WsApiError::deserialization(format!("unknown {union} discriminant: {discriminant}"))
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

/// Source-scrape pins for the decode-boundary conventions.
///
/// The conventions this file establishes — every fixed-size wire field goes
/// through [`fixed_size_field`], every union match ends in
/// [`unknown_union_discriminant`] — are worth nothing if the next decoder added
/// to the crate ignores them. Behavioural tests cannot catch that: they only
/// cover decoders that exist. These scrape the source of every file hosting a
/// `TryFromFbs` impl, so a new decoder that reintroduces either shape fails CI
/// even though nobody wrote a test for it.
///
/// This is what makes "covered by construction" true rather than aspirational.
#[cfg(test)]
mod decode_boundary_conventions {
    /// Every file with a `TryFromFbs` impl. Add new ones here.
    const DECODER_SOURCES: [(&str, &str); 5] = [
        (
            "client_api/client_events.rs",
            include_str!("client_api/client_events.rs"),
        ),
        (
            "contract_interface/update.rs",
            include_str!("contract_interface/update.rs"),
        ),
        (
            "contract_interface/key.rs",
            include_str!("contract_interface/key.rs"),
        ),
        (
            "delegate_interface.rs",
            include_str!("delegate_interface.rs"),
        ),
        ("versioning.rs", include_str!("versioning.rs")),
    ];

    /// Lines of real code, with `//` comments dropped — the prose in this crate
    /// discusses `unreachable!()` at length and must not trip the scan.
    fn code_lines(src: &str) -> impl Iterator<Item = (usize, &str)> {
        src.lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.split("//").next().unwrap_or("")))
            .filter(|(_, l)| !l.trim().is_empty())
    }

    /// No wire field may be read with a conversion that panics on the wrong
    /// length. `.bytes()` is the flatbuffers accessor for a wire vector, so a
    /// line combining it with `unwrap` or `copy_from_slice` is the shape that
    /// produced four of this change's nine bugs.
    ///
    /// The needle is split so this test cannot match its own source if these
    /// files are ever scraped by a future pin.
    #[test]
    fn no_wire_field_is_read_with_a_panicking_conversion() {
        let wire = concat!(".by", "tes()");
        let bad = [concat!("unwr", "ap()"), concat!("copy_from_", "slice")];
        let mut hits = vec![];
        for (name, src) in DECODER_SOURCES {
            for (line_no, line) in code_lines(src) {
                if line.contains(wire) && bad.iter().any(|b| line.contains(b)) {
                    hits.push(format!("{name}:{line_no}: {}", line.trim()));
                }
            }
        }
        assert!(
            hits.is_empty(),
            "a wire field is read with a conversion that panics on the wrong length. \
             The flatbuffers verifier checks that a `(required)` field is PRESENT, not \
             that it is the right LENGTH, so this is reachable from any client and kills \
             the connection task. Use `client_api::fixed_size_field` instead.\n{}",
            hits.join("\n")
        );
    }

    /// No union match may treat an unrecognized discriminant as impossible.
    /// Every generated union verifier ends in `_ => Ok(())`, so it is not.
    #[test]
    fn union_matches_never_use_unreachable() {
        let needle = concat!("unreach", "able!");
        let mut hits = vec![];
        for (name, src) in DECODER_SOURCES {
            for (line_no, line) in code_lines(src) {
                if line.contains(needle) {
                    hits.push(format!("{name}:{line_no}: {}", line.trim()));
                }
            }
        }
        assert!(
            hits.is_empty(),
            "a decoder treats an unrecognized value as impossible. Every generated \
             flatbuffers union verifier ends in `_ => Ok(())`, so any discriminant a \
             client sets reaches the match and this panics the connection task. Return \
             `client_api::unknown_union_discriminant` instead.\n{}",
            hits.join("\n")
        );
    }
}
