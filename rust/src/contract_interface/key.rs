//! Contract key types and identifiers.
//!
//! This module provides the core types for identifying contracts:
//! - `ContractInstanceId`: The hash of contract code and parameters (use for routing/lookup)
//! - `ContractKey`: A complete key specification with code hash (use for storage/execution)

use std::borrow::Borrow;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

use blake3::{traits::digest::Digest, Hasher as Blake3};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::client_api::{TryFromFbs, WsApiError};
use crate::code_hash::CodeHash;
use crate::common_generated::common::ContractKey as FbsContractKey;
use crate::parameters::Parameters;

use super::code::ContractCode;
use super::CONTRACT_KEY_SIZE;

/// The key representing the hash of the contract executable code hash and a set of `parameters`.
#[serde_as]
#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
#[cfg_attr(
    any(feature = "testing", all(test, any(unix, windows))),
    derive(arbitrary::Arbitrary)
)]
#[repr(transparent)]
pub struct ContractInstanceId(#[serde_as(as = "[_; CONTRACT_KEY_SIZE]")] [u8; CONTRACT_KEY_SIZE]);

impl ContractInstanceId {
    pub fn from_params_and_code<'a>(
        params: impl Borrow<Parameters<'a>>,
        code: impl Borrow<ContractCode<'a>>,
    ) -> Self {
        generate_id(params.borrow(), code.borrow())
    }

    pub const fn new(key: [u8; CONTRACT_KEY_SIZE]) -> Self {
        Self(key)
    }

    /// `Base58` string representation of the `contract id`.
    pub fn encode(&self) -> String {
        bs58::encode(self.0)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Parse a `ContractInstanceId` from its **base58 text** form — the inverse
    /// of [`Self::encode`].
    ///
    /// The input is base58 *characters*, not the id's 32 raw bytes. It takes
    /// `impl AsRef<[u8]>` only so that `&str`, `String` and `&[u8]` of base58
    /// text all work; handing it a raw 32-byte id is a bug, and one that does
    /// not look like a bug at the call site. Use
    /// [`ContractInstanceId::new`] for raw bytes you already hold, or the
    /// crate-internal `instance_id_from_fbs` for bytes off the wire.
    ///
    /// This was called `from_bytes`, documented as "build from the binary
    /// representation". That name and doc caused four decode sites to feed it
    /// raw wire bytes, which panicked on every well-formed request: a random
    /// 32-byte id essentially never consists solely of base58 characters (the
    /// alphabet is 58 of 256 byte values, so the odds are about 2e-21).
    pub fn from_base58(bytes: impl AsRef<[u8]>) -> Result<Self, bs58::decode::Error> {
        let mut spec = [0; CONTRACT_KEY_SIZE];
        bs58::decode(bytes)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .onto(&mut spec)?;
        Ok(Self(spec))
    }

    /// Renamed to [`Self::from_base58`], which says what it actually does.
    ///
    /// Kept as a delegating alias so the rename is not a breaking change.
    #[deprecated(
        since = "0.8.5",
        note = "renamed to `from_base58`: this parses base58 TEXT, not raw bytes. \
                For a raw 32-byte id use `ContractInstanceId::new`."
    )]
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, bs58::decode::Error> {
        Self::from_base58(bytes)
    }
}

impl Deref for ContractInstanceId {
    type Target = [u8; CONTRACT_KEY_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ContractInstanceId {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ContractInstanceId::from_base58(s)
    }
}

impl TryFrom<String> for ContractInstanceId {
    type Error = bs58::decode::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        ContractInstanceId::from_base58(s)
    }
}

impl Display for ContractInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl std::fmt::Debug for ContractInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ContractInstanceId")
            .field(&self.encode())
            .finish()
    }
}

/// A complete key specification, that represents a cryptographic hash that identifies the contract.
///
/// This type always contains both the instance ID and the code hash.
/// Use `ContractInstanceId` for operations that only need to identify the contract
/// (routing, client requests), and `ContractKey` for operations that need the full
/// specification (storage, execution).
#[serde_as]
#[derive(Debug, Eq, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(
    any(feature = "testing", all(test, any(unix, windows))),
    derive(arbitrary::Arbitrary)
)]
pub struct ContractKey {
    instance: ContractInstanceId,
    code: CodeHash,
}

impl ContractKey {
    pub fn from_params_and_code<'a>(
        params: impl Borrow<Parameters<'a>>,
        wasm_code: impl Borrow<ContractCode<'a>>,
    ) -> Self {
        let code = wasm_code.borrow();
        let id = generate_id(params.borrow(), code);
        let code_hash = *code.hash();
        Self {
            instance: id,
            code: code_hash,
        }
    }

    /// Gets the whole spec key hash.
    pub fn as_bytes(&self) -> &[u8] {
        self.instance.0.as_ref()
    }

    /// Returns the hash of the contract code.
    pub fn code_hash(&self) -> &CodeHash {
        &self.code
    }

    /// Returns the encoded hash of the contract code.
    pub fn encoded_code_hash(&self) -> String {
        bs58::encode(self.code.0)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    /// Returns the contract key from the encoded hash of the contract code and the given
    /// parameters.
    pub fn from_params(
        code_hash: impl Into<String>,
        parameters: Parameters,
    ) -> Result<Self, bs58::decode::Error> {
        let mut code_key = [0; CONTRACT_KEY_SIZE];
        bs58::decode(code_hash.into())
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .onto(&mut code_key)?;

        let mut hasher = Blake3::new();
        hasher.update(code_key.as_slice());
        hasher.update(parameters.as_ref());
        let full_key_arr = hasher.finalize();

        let mut spec = [0; CONTRACT_KEY_SIZE];
        spec.copy_from_slice(&full_key_arr);
        Ok(Self {
            instance: ContractInstanceId(spec),
            code: CodeHash(code_key),
        })
    }

    /// Returns the `Base58` encoded string of the [`ContractInstanceId`](ContractInstanceId).
    pub fn encoded_contract_id(&self) -> String {
        self.instance.encode()
    }

    pub fn id(&self) -> &ContractInstanceId {
        &self.instance
    }

    /// Constructs a ContractKey from a pre-computed instance ID and code hash.
    ///
    /// This is useful when the node needs to reconstruct a key from stored index data.
    /// Callers must ensure the instance_id was correctly derived from the code_hash
    /// and parameters, as this constructor does not verify consistency.
    pub fn from_id_and_code(instance_id: ContractInstanceId, code_hash: CodeHash) -> Self {
        Self {
            instance: instance_id,
            code: code_hash,
        }
    }
}

impl PartialEq for ContractKey {
    fn eq(&self, other: &Self) -> bool {
        self.instance == other.instance
    }
}

impl std::hash::Hash for ContractKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.instance.0.hash(state);
    }
}

impl From<ContractKey> for ContractInstanceId {
    fn from(key: ContractKey) -> Self {
        key.instance
    }
}

impl Deref for ContractKey {
    type Target = [u8; CONTRACT_KEY_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.instance.0
    }
}

impl std::fmt::Display for ContractKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.instance.fmt(f)
    }
}

/// Error text for a `ContractKey` whose wire `code` field cannot be read as a
/// contract code hash.
///
/// `observed` is the field's byte length, or `None` when the optional field was
/// omitted entirely. Both cases share one message because they fail for the same
/// reason and have the same remedy.
///
/// The old text was `CodeHash::try_from`'s `io::ErrorKind::InvalidData`, which
/// stringifies to "invalid data": nothing a browser developer can act on. Name
/// the field, the expected length, what actually arrived, and why it matters.
///
/// Be precise about WHY, because the obvious phrasing is wrong. The node does
/// NOT resolve a contract's WASM by code hash. It resolves by INSTANCE id:
/// `ContractStore::fetch_contract` recovers the real hash from
/// `key_to_code_part[key.id()]`, the module cache keys on `ContractKey` whose
/// `Hash`/`Eq` use only the instance, and the state stores key on
/// `as_bytes()`, which is instance bytes. The hash is load-bearing at exactly
/// one place: the gate that asks whether this node already holds the code
/// blob. Saying otherwise would tell a reader the hash is fundamentally
/// required, which is the opposite of what freenet-core#4978 argues, and #4978
/// is the fix we point them at.
fn code_hash_error(observed: Option<usize>) -> String {
    let seen = match observed {
        Some(len) => format!("got {len} bytes"),
        None => "the field was absent".to_string(),
    };
    format!(
        "ContractKey.code must be the {CONTRACT_KEY_SIZE}-byte contract code hash; {seen}. \
         An UPDATE cannot be addressed by instance id alone: the node gates an UPDATE on \
         already holding the contract's code blob and probes for it by code hash, so a \
         missing or wrong-length hash is rejected here instead of failing later as \
         \"missing contract: <key>\". Build the key with both parts: in the TypeScript SDK \
         use `new ContractKey(instance, code)` rather than \
         `ContractKey.fromInstanceId(...)`. See freenet/freenet-core#4978."
    )
}

/// Decode a wire `ContractInstanceId`'s bytes, rejecting a wrong length instead
/// of panicking.
///
/// **This is the only correct way to turn wire bytes into a
/// `ContractInstanceId`.** The wire carries the id as 32 RAW bytes — every
/// encode site writes `key.as_bytes()` and the TypeScript SDK writes
/// `Array.from(instance)` — so the decode is a length-checked pass-through and
/// nothing else. In particular do NOT reach for
/// [`ContractInstanceId::from_base58`]: that is a base58 *string* decoder, and
/// pointing it at raw bytes is what broke `related_to`/`instance_id` decoding
/// (it panicked on every well-formed request, because a random 32-byte id
/// essentially never consists solely of base58 characters).
///
/// `field` is the schema path being decoded, so a GET, an UPDATE and a related
/// contract each name their own field in the error.
pub(crate) fn instance_id_from_fbs(
    field: &str,
    data: &[u8],
) -> Result<ContractInstanceId, WsApiError> {
    crate::client_api::fixed_size_field::<CONTRACT_KEY_SIZE>(field, data)
        .map(ContractInstanceId::new)
}

impl<'a> TryFromFbs<&FbsContractKey<'a>> for ContractKey {
    fn try_decode_fbs(key: &FbsContractKey<'a>) -> Result<Self, WsApiError> {
        let instance = instance_id_from_fbs("ContractKey.instance", key.instance().data().bytes())?;
        // The `code` field carries the already-computed 32-byte code hash
        // (BLAKE3 of the wasm), so pass those bytes straight through. Calling
        // `CodeHash::from_code` here would hash the hash again -
        // BLAKE3(BLAKE3(wasm)): yielding a key that never matches the store and
        // breaking every FlatBuffers UpdateRequest ("Contract not in store and
        // no code provided"). GET/SUBSCRIBE dodged this because they decode only
        // the instance id; UPDATE decodes the full key. The delegate decoder
        // already does the pass-through correctly (see
        // `DelegateKey::try_decode_fbs`). Regression test below.
        //
        // Anything other than exactly `CONTRACT_KEY_SIZE` bytes is rejected here,
        // even though the schema marks `code` optional and the TypeScript SDK's
        // `ContractKey.fromInstanceId(...)` emits a present-but-empty vector.
        // That is a deliberate narrowing, not an oversight: `try_decode_fbs` is
        // reached only from the UPDATE decode path, and an UPDATE genuinely needs
        // the hash today: freenet-core gates on `code_blob_stored(key.code_hash())`
        // and, because UPDATE supplies no contract code, a miss fails the request
        // with "Contract not in store and no code provided". So an empty or absent
        // `code` has never produced a working UPDATE; before this change it merely
        // failed later, at the store gate, with an error pointing at the node
        // rather than at the caller. Rejecting at the boundary with an actionable
        // message is strictly better diagnostics for the same outcome.
        //
        // The honest end state is `Option<CodeHash>` threaded through
        // `ContractRequest::Update`, once freenet-core resolves a `None` from the
        // instance id the way GET and SUBSCRIBE already do via
        // `code_hash_from_id`. That is tracked in freenet/freenet-core#4978; it
        // needs a coordinated stdlib + core change, so it is not done here.
        let code_bytes = key
            .code()
            .map(|code_hash| code_hash.bytes())
            .ok_or_else(|| WsApiError::deserialization(code_hash_error(None)))?;
        let code = CodeHash::try_from(code_bytes)
            .map_err(|_| WsApiError::deserialization(code_hash_error(Some(code_bytes.len()))))?;
        Ok(ContractKey { instance, code })
    }
}

fn generate_id<'a>(
    parameters: &Parameters<'a>,
    code_data: &ContractCode<'a>,
) -> ContractInstanceId {
    let contract_hash = code_data.hash();

    let mut hasher = Blake3::new();
    hasher.update(contract_hash.0.as_slice());
    hasher.update(parameters.as_ref());
    let full_key_arr = hasher.finalize();

    debug_assert_eq!(full_key_arr[..].len(), CONTRACT_KEY_SIZE);
    let mut spec = [0; CONTRACT_KEY_SIZE];
    spec.copy_from_slice(&full_key_arr);
    ContractInstanceId(spec)
}

#[inline]
pub(super) fn internal_fmt_key(
    key: &[u8; CONTRACT_KEY_SIZE],
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let r = bs58::encode(key)
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_string();
    write!(f, "{}", &r[..8])
}

#[cfg(test)]
mod fbs_tests {
    use super::*;
    use crate::common_generated::common::{
        ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs, ContractKeyArgs,
    };

    /// The wire `code` field carries the raw 32-byte code hash, and the decoder
    /// must return those exact bytes. Regression for the double-hash bug where
    /// `try_decode_fbs` re-hashed the hash (BLAKE3(BLAKE3(wasm))), producing a
    /// key that never matched the store and failing every FlatBuffers
    /// UpdateRequest with "Contract not in store and no code provided".
    #[test]
    fn contract_key_code_hash_passes_through_fbs_decode() {
        // A distinct, arbitrary code hash. The decoder must reproduce it
        // verbatim; if it re-hashes, the assertion below fails.
        let code_bytes = [42u8; CONTRACT_KEY_SIZE];
        let decoded = decode_with_code(Some(&code_bytes)).expect("decode ContractKey");

        assert_eq!(
            decoded.code_hash().as_ref(),
            &code_bytes,
            "decoder must pass the code hash through unchanged, not re-hash it"
        );
        assert_eq!(decoded.id().as_bytes(), &[7u8; CONTRACT_KEY_SIZE]);
    }

    /// Build a `ContractKey` flatbuffer whose `code` vector holds `code_bytes`,
    /// or which omits the optional `code` field entirely when `code_bytes` is
    /// `None`, and run it through the decoder.
    fn decode_with_code(code_bytes: Option<&[u8]>) -> Result<ContractKey, WsApiError> {
        decode_key(&[7u8; CONTRACT_KEY_SIZE], code_bytes)
    }

    /// Serialize a `ContractKey` flatbuffer with the given raw field bytes.
    /// Neither vector is length-checked here on purpose: the point is to feed
    /// the decoder exactly what a peer could put on the wire.
    fn encode_key(instance_bytes: &[u8], code_bytes: Option<&[u8]>) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let instance_data = builder.create_vector(instance_bytes);
        let instance_offset = FbsContractInstanceId::create(
            &mut builder,
            &ContractInstanceIdArgs {
                data: Some(instance_data),
            },
        );
        let code = code_bytes.map(|bytes| builder.create_vector(bytes));
        let key_offset = FbsContractKey::create(
            &mut builder,
            &ContractKeyArgs {
                instance: Some(instance_offset),
                code,
            },
        );
        builder.finish_minimal(key_offset);
        builder.finished_data().to_vec()
    }

    fn decode_key(
        instance_bytes: &[u8],
        code_bytes: Option<&[u8]>,
    ) -> Result<ContractKey, WsApiError> {
        let bytes = encode_key(instance_bytes, code_bytes);
        let fbs_key =
            flatbuffers::root::<FbsContractKey>(&bytes).expect("valid ContractKey flatbuffer");
        ContractKey::try_decode_fbs(&fbs_key)
    }

    /// FIXED BYTES pinning the `common.ContractKey` vtable layout.
    ///
    /// Why a blob when the sibling tests build programmatically: a
    /// programmatic test encodes with the SAME generated code it then decodes
    /// with, so a `common.fbs` change that reorders `instance` and `code` moves
    /// both sides together and the test stays green while real TypeScript
    /// clients break. Only bytes frozen OUTSIDE the generated code catch that.
    /// This restores a property the PR briefly lost: after rebuilding the
    /// update fixture programmatically, nothing decoded a `common.ContractKey`
    /// from fixed bytes at all (the PUT fixture recomputes its key from
    /// params+code and never reads the table; GET/SUBSCRIBE read only instance
    /// bytes). Follows `delegate_interface.rs`'s `*_wire_format_is_stable`.
    ///
    /// Distinct from the TypeScript-blob test in `client_api::client_events`,
    /// which pins the REJECT path. This one must pin a SUCCESSFUL decode.
    ///
    /// To regenerate after a deliberate schema change: build the same key with
    /// `encode_key(&[7; 32], Some(&[42; 32]))` and paste `finished_data()`.
    /// Changing these bytes is a wire-format break; be sure that is intended.
    const CONTRACT_KEY_WIRE_FORMAT: &[u8] = &[
        12, 0, 0, 0, 8, 0, 12, 0, 4, 0, 8, 0, 8, 0, 0, 0, 52, 0, 0, 0, 4, 0, 0, 0, 32, 0, 0, 0, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 0, 0, 6, 0, 8, 0, 4, 0, 6, 0, 0, 0, 4, 0, 0, 0, 32, 0, 0,
        0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7, 7,
    ];

    #[test]
    fn contract_key_wire_format_is_stable() {
        let fbs_key = flatbuffers::root::<FbsContractKey>(CONTRACT_KEY_WIRE_FORMAT)
            .expect("the pinned ContractKey bytes must still parse");
        let decoded = ContractKey::try_decode_fbs(&fbs_key)
            .expect("the pinned ContractKey bytes must still decode");
        assert_eq!(
            decoded.id().as_bytes(),
            &[7u8; CONTRACT_KEY_SIZE],
            "instance id moved: `common.ContractKey`'s vtable layout changed, \
             which breaks every already-deployed client"
        );
        assert_eq!(
            decoded.code_hash().as_ref(),
            &[42u8; CONTRACT_KEY_SIZE],
            "code hash moved: `common.ContractKey`'s vtable layout changed, \
             which breaks every already-deployed client"
        );
    }

    /// A wrong-length `instance` is rejected, not panicked on.
    ///
    /// `instance`/`data` are `(required)` in the schema, but the flatbuffers
    /// verifier checks PRESENCE, not LENGTH: so `flatbuffers::root` accepts an
    /// 8-byte instance and the `try_into().unwrap()` this replaced then panicked
    /// with `TryFromSliceError`. Nothing catches unwind on this path and
    /// `panic = "abort"` is not set, so it killed the client's connection task:
    /// a remote, wire-reachable panic reachable from UPDATE, GET and SUBSCRIBE.
    #[test]
    fn contract_key_decode_rejects_short_instance_without_panicking() {
        let err = decode_key(&[1u8; 8], Some(&[42u8; CONTRACT_KEY_SIZE]))
            .expect_err("a short instance vector must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("ContractKey.instance") && msg.contains("got 8 bytes"),
            "the error must name the field and the observed length, got: {msg}"
        );
    }

    /// An over-long `instance` is rejected too: the guard is a length equality,
    /// not a minimum, so a peer cannot pad its way past it.
    #[test]
    fn contract_key_decode_rejects_long_instance() {
        let err = decode_key(&[1u8; 64], Some(&[42u8; CONTRACT_KEY_SIZE]))
            .expect_err("an over-long instance vector must be rejected");
        assert!(err.to_string().contains("got 64 bytes"), "got: {err}");
    }

    /// A zero-length `code` is rejected, and the error says what to do about it.
    ///
    /// This pins BOTH halves of the decision, because each is easy to undo
    /// without noticing the other:
    ///
    /// 1. The rejection itself. The schema marks `code` optional and the
    ///    TypeScript SDK's `ContractKey.fromInstanceId(...)` emits exactly this
    ///    shape, so narrowing to "32 bytes or nothing" is a deliberate call, not
    ///    an accident of using `CodeHash::try_from`. It is safe because an
    ///    UPDATE carrying no code hash could never be served anyway: the node
    ///    resolves the WASM by code hash and fails at the store gate.
    /// 2. The message. The whole point of rejecting early is diagnosis: the
    ///    previous text was `io::ErrorKind::InvalidData`, which stringifies to
    ///    "invalid data" and told a browser developer nothing. Reverting to a
    ///    bare `try_from` error would keep this test's first assertion green
    ///    while destroying the reason the change was made, so the message
    ///    content is asserted too.
    #[test]
    fn contract_key_decode_rejects_empty_code_with_actionable_error() {
        let err = decode_with_code(Some(&[])).expect_err("empty code must be rejected");
        let msg = err.to_string();

        assert!(
            msg.contains("ContractKey.code"),
            "error must name the offending field, got: {msg}"
        );
        assert!(
            msg.contains("32-byte"),
            "error must state the expected length, got: {msg}"
        );
        assert!(
            msg.contains("got 0 bytes"),
            "error must state the actual length, got: {msg}"
        );
        assert!(
            msg.contains("instance id alone"),
            "error must explain why the hash is required, got: {msg}"
        );
        assert!(
            msg.contains("4978"),
            "error must point at the tracking issue for the real fix, got: {msg}"
        );
    }

    /// An absent `code` is rejected the same way. Unreachable from any
    /// first-party producer: the TypeScript `pack()` always emits the vector
    /// and the Rust stdlib has no client-to-node FBS request encoder: so this
    /// only guards hand-rolled third-party encoders. Behavior is unchanged from
    /// before this PR (it was already rejected); only the message improved, so
    /// what is pinned here is that the two paths stay consistent.
    #[test]
    fn contract_key_decode_rejects_absent_code_with_actionable_error() {
        let err = decode_with_code(None).expect_err("absent code must be rejected");
        let msg = err.to_string();

        assert!(
            msg.contains("ContractKey.code") && msg.contains("4978"),
            "absent-code error must carry the same guidance as the empty case, got: {msg}"
        );
        assert!(
            msg.contains("absent"),
            "absent-code error must distinguish itself from a wrong-length one, got: {msg}"
        );
    }

    /// A wrong-but-nonzero length is rejected with the length it saw. Guards the
    /// obvious partial fix of special-casing only the empty vector.
    #[test]
    fn contract_key_decode_rejects_wrong_length_code() {
        let err = decode_with_code(Some(&[1u8; 16])).expect_err("16-byte code must be rejected");
        assert!(
            err.to_string().contains("got 16 bytes"),
            "error must report the observed length, got: {err}"
        );
    }
}
