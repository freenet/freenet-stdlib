use std::{
    borrow::{Borrow, Cow},
    fmt::Display,
    fs::File,
    io::Read,
    ops::Deref,
    path::Path,
};

use blake3::{traits::digest::Digest, Hasher as Blake3};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::serde_as;

use crate::generated::client_request::{
    DelegateKey as FbsDelegateKey, InboundDelegateMsg as FbsInboundDelegateMsg,
    InboundDelegateMsgType,
};

use crate::common_generated::common::SecretsId as FbsSecretsId;

use crate::client_api::{fixed_size_field, unknown_union_discriminant, TryFromFbs, WsApiError};
use crate::contract_interface::{RelatedContracts, UpdateData, CONTRACT_KEY_SIZE};
use crate::prelude::{ContractInstanceId, WrappedState};
use crate::versioning::ContractContainer;
use crate::{code_hash::CodeHash, prelude::Parameters};

const DELEGATE_HASH_LENGTH: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delegate<'a> {
    #[serde(borrow)]
    parameters: Parameters<'a>,
    #[serde(borrow)]
    pub data: DelegateCode<'a>,
    key: DelegateKey,
}

impl Delegate<'_> {
    pub fn key(&self) -> &DelegateKey {
        &self.key
    }

    pub fn code(&self) -> &DelegateCode<'_> {
        &self.data
    }

    pub fn code_hash(&self) -> &CodeHash {
        &self.data.code_hash
    }

    pub fn params(&self) -> &Parameters<'_> {
        &self.parameters
    }

    pub fn into_owned(self) -> Delegate<'static> {
        Delegate {
            parameters: self.parameters.into_owned(),
            data: self.data.into_owned(),
            key: self.key,
        }
    }

    pub fn size(&self) -> usize {
        self.parameters.size() + self.data.size()
    }

    pub(crate) fn deserialize_delegate<'de, D>(deser: D) -> Result<Delegate<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data: Delegate<'de> = Deserialize::deserialize(deser)?;
        Ok(data.into_owned())
    }
}

impl PartialEq for Delegate<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Delegate<'_> {}

impl<'a> From<(&DelegateCode<'a>, &Parameters<'a>)> for Delegate<'a> {
    fn from((data, parameters): (&DelegateCode<'a>, &Parameters<'a>)) -> Self {
        Self {
            key: DelegateKey::from_params_and_code(parameters, data),
            parameters: parameters.clone(),
            data: data.clone(),
        }
    }
}

/// Executable delegate
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde_as]
pub struct DelegateCode<'a> {
    #[serde_as(as = "serde_with::Bytes")]
    #[serde(borrow)]
    pub(crate) data: Cow<'a, [u8]>,
    // todo: skip serializing and instead compute it
    pub(crate) code_hash: CodeHash,
}

impl DelegateCode<'static> {
    /// Loads the contract raw wasm module, without any version.
    pub fn load_raw(path: &Path) -> Result<Self, std::io::Error> {
        let contract_data = Self::load_bytes(path)?;
        Ok(DelegateCode::from(contract_data))
    }

    pub(crate) fn load_bytes(path: &Path) -> Result<Vec<u8>, std::io::Error> {
        let mut contract_file = File::open(path)?;
        let mut contract_data = if let Ok(md) = contract_file.metadata() {
            Vec::with_capacity(md.len() as usize)
        } else {
            Vec::new()
        };
        contract_file.read_to_end(&mut contract_data)?;
        Ok(contract_data)
    }
}

impl DelegateCode<'_> {
    /// Delegate code hash.
    pub fn hash(&self) -> &CodeHash {
        &self.code_hash
    }

    /// Returns the `Base58` string representation of the delegate key.
    pub fn hash_str(&self) -> String {
        Self::encode_hash(&self.code_hash.0)
    }

    /// Reference to delegate code.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the `Base58` string representation of a hash.
    pub fn encode_hash(hash: &[u8; DELEGATE_HASH_LENGTH]) -> String {
        bs58::encode(hash)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    pub fn into_owned(self) -> DelegateCode<'static> {
        DelegateCode {
            code_hash: self.code_hash,
            data: Cow::from(self.data.into_owned()),
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl PartialEq for DelegateCode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.code_hash == other.code_hash
    }
}

impl Eq for DelegateCode<'_> {}

impl AsRef<[u8]> for DelegateCode<'_> {
    fn as_ref(&self) -> &[u8] {
        self.data.borrow()
    }
}

impl From<Vec<u8>> for DelegateCode<'static> {
    fn from(data: Vec<u8>) -> Self {
        let key = CodeHash::from_code(data.as_slice());
        DelegateCode {
            data: Cow::from(data),
            code_hash: key,
        }
    }
}

impl<'a> From<&'a [u8]> for DelegateCode<'a> {
    fn from(code: &'a [u8]) -> Self {
        let key = CodeHash::from_code(code);
        DelegateCode {
            data: Cow::from(code),
            code_hash: key,
        }
    }
}

#[serde_as]
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct DelegateKey {
    #[serde_as(as = "[_; DELEGATE_HASH_LENGTH]")]
    key: [u8; DELEGATE_HASH_LENGTH],
    code_hash: CodeHash,
}

impl From<DelegateKey> for SecretsId {
    fn from(key: DelegateKey) -> SecretsId {
        SecretsId {
            hash: key.key,
            key: vec![],
        }
    }
}

impl DelegateKey {
    pub const fn new(key: [u8; DELEGATE_HASH_LENGTH], code_hash: CodeHash) -> Self {
        Self { key, code_hash }
    }

    fn from_params_and_code<'a>(
        params: impl Borrow<Parameters<'a>>,
        wasm_code: impl Borrow<DelegateCode<'a>>,
    ) -> Self {
        let code = wasm_code.borrow();
        let key = generate_id(params.borrow(), code);
        Self {
            key,
            code_hash: *code.hash(),
        }
    }

    pub fn encode(&self) -> String {
        bs58::encode(self.key)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    pub fn code_hash(&self) -> &CodeHash {
        &self.code_hash
    }

    pub fn bytes(&self) -> &[u8] {
        self.key.as_ref()
    }

    pub fn from_params(
        code_hash: impl Into<String>,
        parameters: &Parameters,
    ) -> Result<Self, bs58::decode::Error> {
        let mut code_key = [0; DELEGATE_HASH_LENGTH];
        bs58::decode(code_hash.into())
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .onto(&mut code_key)?;
        let mut hasher = Blake3::new();
        hasher.update(code_key.as_slice());
        hasher.update(parameters.as_ref());
        let full_key_arr = hasher.finalize();

        debug_assert_eq!(full_key_arr[..].len(), DELEGATE_HASH_LENGTH);
        let mut key = [0; DELEGATE_HASH_LENGTH];
        key.copy_from_slice(&full_key_arr);

        Ok(Self {
            key,
            code_hash: CodeHash(code_key),
        })
    }
}

impl Deref for DelegateKey {
    type Target = [u8; DELEGATE_HASH_LENGTH];

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl Display for DelegateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl<'a> TryFromFbs<&FbsDelegateKey<'a>> for DelegateKey {
    fn try_decode_fbs(key: &FbsDelegateKey<'a>) -> Result<Self, WsApiError> {
        // Both fields are `(required)` in the schema and BOTH need an explicit
        // length check, because the verifier only guarantees presence. `key`
        // used to be a bare `copy_from_slice` into a `[0; 32]`, which panics on
        // a length mismatch, while `code_hash` one line below was already
        // length-checked inside `CodeHash::try_from`. Keep them symmetric: a
        // future field added here needs the same treatment.
        let key_bytes =
            fixed_size_field::<DELEGATE_HASH_LENGTH>("DelegateKey.key", key.key().bytes())?;
        // `CodeHash::try_from` DOES length-check, so this field never panicked —
        // but its error stringifies to "invalid data", naming neither the field
        // nor the length. Symmetric treatment means the same message shape, not
        // merely the same safety, so it goes through the same helper.
        let code_hash = CodeHash::new(fixed_size_field::<CONTRACT_KEY_SIZE>(
            "DelegateKey.code_hash",
            key.code_hash().bytes(),
        )?);
        Ok(DelegateKey {
            key: key_bytes,
            code_hash,
        })
    }
}

/// Type of errors during interaction with a delegate.
///
/// Marked `#[non_exhaustive]` so future error variants can be added without a
/// source-level break. Downstream `match` sites must include a wildcard arm.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum DelegateError {
    #[error("de/serialization error: {0}")]
    Deser(String),
    #[error("{0}")]
    Other(String),
}

fn generate_id<'a>(
    parameters: &Parameters<'a>,
    code_data: &DelegateCode<'a>,
) -> [u8; DELEGATE_HASH_LENGTH] {
    let contract_hash = code_data.hash();

    let mut hasher = Blake3::new();
    hasher.update(contract_hash.0.as_slice());
    hasher.update(parameters.as_ref());
    let full_key_arr = hasher.finalize();

    debug_assert_eq!(full_key_arr[..].len(), DELEGATE_HASH_LENGTH);
    let mut key = [0; DELEGATE_HASH_LENGTH];
    key.copy_from_slice(&full_key_arr);
    key
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SecretsId {
    #[serde_as(as = "serde_with::Bytes")]
    key: Vec<u8>,
    #[serde_as(as = "[_; 32]")]
    hash: [u8; 32],
}

impl SecretsId {
    pub fn new(key: Vec<u8>) -> Self {
        let mut hasher = Blake3::new();
        hasher.update(&key);
        let hashed = hasher.finalize();
        let mut hash = [0; 32];
        hash.copy_from_slice(&hashed);
        Self { key, hash }
    }

    pub fn encode(&self) -> String {
        bs58::encode(self.hash)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
    pub fn key(&self) -> &[u8] {
        self.key.as_slice()
    }
}

impl Display for SecretsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl<'a> TryFromFbs<&FbsSecretsId<'a>> for SecretsId {
    fn try_decode_fbs(key: &FbsSecretsId<'a>) -> Result<Self, WsApiError> {
        // No production caller reaches this decoder today — `common.SecretsId`
        // appears in no client-request table. It is fixed anyway because the
        // `copy_from_slice` it replaces is a loaded gun for whoever wires it up:
        // `hash` is `(required)`, which the verifier reads as "present", not
        // "32 bytes", so the first client to send a short one would have
        // panicked the connection task.
        let key_hash = fixed_size_field::<32>("SecretsId.hash", key.hash().bytes())?;
        Ok(SecretsId {
            key: key.key().bytes().to_vec(),
            hash: key_hash,
        })
    }
}

/// Identifies where an inbound application message originated from.
///
/// When a web app sends a message to a delegate through the WebSocket API with
/// an authentication token, the runtime resolves the token to the originating
/// contract and wraps it in `MessageOrigin::WebApp`. When one delegate sends a
/// message to another via [`OutboundDelegateMsg::SendDelegateMessage`], the
/// runtime attests the caller's identity in `MessageOrigin::Delegate`.
/// Delegates receive this as the `origin` parameter of
/// [`DelegateInterface::process`].
///
/// This enum is `#[non_exhaustive]`: downstream code matching on it must
/// include a wildcard arm so future variants can be added without a
/// source-level breaking change.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageOrigin {
    /// The message was sent by a web application backed by the given contract.
    WebApp(ContractInstanceId),
    /// The message was sent by another delegate via
    /// [`OutboundDelegateMsg::SendDelegateMessage`]. The carried key is the
    /// runtime-attested identity of the calling delegate; the receiver can
    /// trust it to make authorization decisions.
    ///
    /// Note: an inter-delegate message **replaces** rather than composes with
    /// any inherited `WebApp` origin the calling delegate may itself hold.
    /// The receiver sees only `Delegate(caller_key)` for the duration of the
    /// call, and does not gain contract access on behalf of any web app the
    /// caller was acting for. Authorization should be made on the calling
    /// delegate's identity alone.
    Delegate(DelegateKey),
}

/// A Delegate is a webassembly code designed to act as an agent for the user on
/// Freenet. Delegates can:
///
///  * Store private data on behalf of the user
///  * Create, read, and modify contracts
///  * Create other delegates
///  * Send and receive messages from other delegates and user interfaces
///  * Ask the user questions and receive answers
///
/// Example use cases:
///
///  * A delegate stores a private key for the user, other components can ask
///    the delegate to sign messages, it will ask the user for permission
///  * A delegate monitors an inbox contract and downloads new messages when
///    they arrive
///
/// # Example
///
/// ```ignore
/// use freenet_stdlib::prelude::*;
///
/// struct MyDelegate;
///
/// #[delegate]
/// impl DelegateInterface for MyDelegate {
///     fn process(
///         ctx: &mut DelegateCtx,
///         _params: Parameters<'static>,
///         _origin: Option<MessageOrigin>,
///         message: InboundDelegateMsg,
///     ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
///         // Access secrets synchronously - no round-trip needed!
///         if let Some(key) = ctx.get_secret(b"private_key") {
///             // use key...
///         }
///         ctx.set_secret(b"new_key", b"value");
///
///         // Read/write context for temporary state within a batch
///         ctx.write(b"some state");
///
///         Ok(vec![])
///     }
/// }
/// ```
pub trait DelegateInterface {
    /// Process inbound message, producing zero or more outbound messages in response.
    ///
    /// # Arguments
    /// - `ctx`: Mutable handle to the delegate's execution environment. Provides:
    ///   - **Context** (temporary): `read()`, `write()`, `len()`, `clear()` - state within a batch
    ///   - **Secrets** (persistent): `get_secret()`, `set_secret()`, `has_secret()`, `remove_secret()`
    /// - `parameters`: The delegate's initialization parameters.
    /// - `origin`: An optional [`MessageOrigin`] identifying where the message came from.
    ///   For messages sent by web applications, this is `MessageOrigin::WebApp(contract_id)`.
    /// - `message`: The inbound message to process.
    fn process(
        ctx: &mut crate::delegate_host::DelegateCtx,
        parameters: Parameters<'static>,
        origin: Option<MessageOrigin>,
        message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError>;
}

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegateContext(#[serde_as(as = "serde_with::Bytes")] Vec<u8>);

impl DelegateContext {
    pub const MAX_SIZE: usize = 4096 * 10 * 10;

    pub fn new(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() < Self::MAX_SIZE);
        Self(bytes)
    }

    pub fn append(&mut self, bytes: &mut Vec<u8>) {
        assert!(self.0.len() + bytes.len() < Self::MAX_SIZE);
        self.0.append(bytes)
    }

    pub fn replace(&mut self, bytes: Vec<u8>) {
        assert!(bytes.len() < Self::MAX_SIZE);
        let _ = std::mem::replace(&mut self.0, bytes);
    }
}

impl AsRef<[u8]> for DelegateContext {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Messages delivered **into** a delegate's `process()` function.
///
/// This is the inbound counterpart of [`OutboundDelegateMsg`] and sits on the
/// host↔delegate wire boundary.
///
/// Marked `#[non_exhaustive]` so future variants can be added without a
/// source-level break; downstream `match` sites must include a wildcard arm.
/// [`OutboundDelegateMsg`] is deliberately **not** marked, and the asymmetry is
/// the point — see the rationale on that enum. (An earlier version of this
/// comment asserted that `OutboundDelegateMsg` already carried the attribute.
/// It never has.)
///
/// # Wire format and compatibility
///
/// bincode, variant index 0..=N in **declaration order**. Two rules follow, and
/// the compiler enforces neither:
///
/// - **Never insert or reorder a variant.** That silently reassigns every later
///   tag, so delegate WASM compiled against an older stdlib decodes the same
///   bytes into a *different* variant — no error, just a message quietly
///   reinterpreted as another one. `delegate_msg_variant_tags_are_pinned` pins
///   the tag of every variant of both enums so a reorder fails CI instead.
/// - **Appending is compatible in exactly one direction.** An old sender's old
///   variant always decodes on a new receiver. A **new** sender's **new**
///   variant does **not** decode on an old receiver: bincode rejects the
///   unknown tag — as `ErrorKind::Custom("invalid value: integer `N`, expected
///   variant index 0 <= i < M")`, since bincode hands the index to serde's
///   derived visitor rather than validating it itself. (Not
///   `InvalidTagEncoding`, which bincode only ever produces for a bad `Option`
///   discriminant.) `#[non_exhaustive]` does
///   not change this — it is a source-level attribute with no effect on the
///   encoding, and serde has no unknown-variant fallback to fall back to.
///
/// For this enum the incompatible direction is a **new host → old delegate**,
/// and it is mostly unreachable in practice: the host emits a response variant
/// only in reply to the matching request variant, so a delegate that never
/// emits a request added in stdlib version X never receives the response added
/// in X. Deployed delegate WASM therefore keeps working against an upgraded
/// node. The genuinely constrained direction is delegate → host; see
/// [`OutboundDelegateMsg`].
///
/// The compatibility claims above are asserted, not merely asserted-in-prose,
/// by the `delegate_wire_compat` test module at the bottom of this file.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InboundDelegateMsg<'a> {
    ApplicationMessage(ApplicationMessage),
    UserResponse(#[serde(borrow)] UserInputResponse<'a>),
    GetContractResponse(GetContractResponse),
    PutContractResponse(PutContractResponse),
    UpdateContractResponse(UpdateContractResponse),
    SubscribeContractResponse(SubscribeContractResponse),
    ContractNotification(ContractNotification),
    DelegateMessage(DelegateMessage),
    // Appended in 0.9.0 at tag 8. New variants go at the END, never inserted —
    // see the wire-format note on this enum.
    UnsubscribeContractResponse(UnsubscribeContractResponse),
}

impl InboundDelegateMsg<'_> {
    pub fn into_owned(self) -> InboundDelegateMsg<'static> {
        match self {
            InboundDelegateMsg::ApplicationMessage(r) => InboundDelegateMsg::ApplicationMessage(r),
            InboundDelegateMsg::UserResponse(r) => InboundDelegateMsg::UserResponse(r.into_owned()),
            InboundDelegateMsg::GetContractResponse(r) => {
                InboundDelegateMsg::GetContractResponse(r)
            }
            InboundDelegateMsg::PutContractResponse(r) => {
                InboundDelegateMsg::PutContractResponse(r)
            }
            InboundDelegateMsg::UpdateContractResponse(r) => {
                InboundDelegateMsg::UpdateContractResponse(r)
            }
            InboundDelegateMsg::SubscribeContractResponse(r) => {
                InboundDelegateMsg::SubscribeContractResponse(r)
            }
            InboundDelegateMsg::ContractNotification(r) => {
                InboundDelegateMsg::ContractNotification(r)
            }
            InboundDelegateMsg::DelegateMessage(r) => InboundDelegateMsg::DelegateMessage(r),
            InboundDelegateMsg::UnsubscribeContractResponse(r) => {
                InboundDelegateMsg::UnsubscribeContractResponse(r)
            }
        }
    }

    pub fn get_context(&self) -> Option<&DelegateContext> {
        match self {
            InboundDelegateMsg::ApplicationMessage(ApplicationMessage { context, .. }) => {
                Some(context)
            }
            // UserResponse carries a context too. It was missing from both
            // accessors, so this returned None for it — the `_ => None`
            // wildcard below swallowed the omission silently. Found in review.
            InboundDelegateMsg::UserResponse(UserInputResponse { context, .. }) => Some(context),
            InboundDelegateMsg::GetContractResponse(GetContractResponse { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::PutContractResponse(PutContractResponse { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::UpdateContractResponse(UpdateContractResponse {
                context, ..
            }) => Some(context),
            InboundDelegateMsg::SubscribeContractResponse(SubscribeContractResponse {
                context,
                ..
            }) => Some(context),
            InboundDelegateMsg::ContractNotification(ContractNotification { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::DelegateMessage(DelegateMessage { context, .. }) => Some(context),
            InboundDelegateMsg::UnsubscribeContractResponse(UnsubscribeContractResponse {
                context,
                ..
            }) => Some(context),
            // No wildcard, deliberately. Every variant carries a context, and
            // the `_ => None` that used to sit here is what let UserResponse go
            // unhandled and silently report "no context". Exhaustive means a
            // new variant is a compile error here instead.
        }
    }

    pub fn get_mut_context(&mut self) -> Option<&mut DelegateContext> {
        match self {
            InboundDelegateMsg::ApplicationMessage(ApplicationMessage { context, .. }) => {
                Some(context)
            }
            // UserResponse carries a context too. It was missing from both
            // accessors, so this returned None for it — the `_ => None`
            // wildcard below swallowed the omission silently. Found in review.
            InboundDelegateMsg::UserResponse(UserInputResponse { context, .. }) => Some(context),
            InboundDelegateMsg::GetContractResponse(GetContractResponse { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::PutContractResponse(PutContractResponse { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::UpdateContractResponse(UpdateContractResponse {
                context, ..
            }) => Some(context),
            InboundDelegateMsg::SubscribeContractResponse(SubscribeContractResponse {
                context,
                ..
            }) => Some(context),
            InboundDelegateMsg::ContractNotification(ContractNotification { context, .. }) => {
                Some(context)
            }
            InboundDelegateMsg::DelegateMessage(DelegateMessage { context, .. }) => Some(context),
            InboundDelegateMsg::UnsubscribeContractResponse(UnsubscribeContractResponse {
                context,
                ..
            }) => Some(context),
            // No wildcard, deliberately. Every variant carries a context, and
            // the `_ => None` that used to sit here is what let UserResponse go
            // unhandled and silently report "no context". Exhaustive means a
            // new variant is a compile error here instead.
        }
    }
}

impl From<ApplicationMessage> for InboundDelegateMsg<'_> {
    fn from(value: ApplicationMessage) -> Self {
        Self::ApplicationMessage(value)
    }
}

impl<'a> TryFromFbs<&FbsInboundDelegateMsg<'a>> for InboundDelegateMsg<'a> {
    fn try_decode_fbs(msg: &FbsInboundDelegateMsg<'a>) -> Result<Self, WsApiError> {
        match msg.inbound_type() {
            InboundDelegateMsgType::common_ApplicationMessage => {
                let app_msg = msg.inbound_as_common_application_message().unwrap();
                let app_msg = ApplicationMessage {
                    payload: app_msg.payload().bytes().to_vec(),
                    context: DelegateContext::new(app_msg.context().bytes().to_vec()),
                    processed: app_msg.processed(),
                };
                Ok(InboundDelegateMsg::ApplicationMessage(app_msg))
            }
            InboundDelegateMsgType::UserInputResponse => {
                let user_response = msg.inbound_as_user_input_response().unwrap();
                let user_response = UserInputResponse {
                    request_id: user_response.request_id(),
                    response: ClientResponse::new(user_response.response().data().bytes().to_vec()),
                    context: DelegateContext::new(
                        user_response.delegate_context().bytes().to_vec(),
                    ),
                };
                Ok(InboundDelegateMsg::UserResponse(user_response))
            }
            // Reachable, not `unreachable!()`: the generated verifier for this
            // union ends in `_ => Ok(())`, so any discriminant a client sets —
            // including `NONE` — arrives here. See `unknown_union_discriminant`.
            other => Err(unknown_union_discriminant(
                "InboundDelegateMsgType",
                other.0,
            )),
        }
    }
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationMessage {
    pub payload: Vec<u8>,
    pub context: DelegateContext,
    pub processed: bool,
}

impl ApplicationMessage {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            payload,
            context: DelegateContext::default(),
            processed: false,
        }
    }

    pub fn with_context(mut self, context: DelegateContext) -> Self {
        self.context = context;
        self
    }

    pub fn processed(mut self, p: bool) -> Self {
        self.processed = p;
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInputResponse<'a> {
    pub request_id: u32,
    #[serde(borrow)]
    pub response: ClientResponse<'a>,
    pub context: DelegateContext,
}

impl UserInputResponse<'_> {
    pub fn into_owned(self) -> UserInputResponse<'static> {
        UserInputResponse {
            request_id: self.request_id,
            response: self.response.into_owned(),
            context: self.context,
        }
    }
}

/// Messages emitted **out of** a delegate's `process()` function.
///
/// This is the outbound counterpart of [`InboundDelegateMsg`] and sits on the
/// same host↔delegate wire boundary.
///
/// # Deliberately not `#[non_exhaustive]`
///
/// Adding a variant here is a source-level break for any downstream crate that
/// matches on it exhaustively. That is the intended behaviour and it should not
/// be "fixed" by marking the enum.
///
/// Every variant of this enum is a **request the host must act on**. There is
/// one host — freenet-core — and it dispatches these in exhaustive matches with
/// no wildcard (`crates/core/src/contract.rs`, in the request loop and again in
/// the app-message filter). Marking this enum `#[non_exhaustive]` would force
/// those matches to grow `_ =>` arms, and a newly added variant would then
/// compile against the host with **no arm of its own**: the delegate's request
/// would fall into the wildcard, the call would appear to succeed, and nothing
/// would report that it did nothing.
///
/// The compile error is what stops that, and it is the only mechanism that
/// does. Keep it.
///
/// Two honest limits on this argument, because it is easy to claim more:
///
/// - **It forces an arm to exist, not a handler to be correct.** This crate's
///   own FlatBuffers encoder (`client_api::client_events`) has explicit arms
///   for five outbound variants that log an error and drop the message. The
///   compile error made someone write those arms deliberately; it could not
///   make them do anything useful.
/// - **It is not the bug behind this workstream.** A delegate
///   `SubscribeContractRequest` *is* handled by the host today. Its defect is
///   different and subtler: it registers no demand in the network, so the
///   subscription does not pin the contract (freenet-core#4669). Do not read
///   the compile-error argument as a fix for that; it is a guard against a
///   different failure that has not happened yet, which is the point of a
///   guard.
///
/// [`InboundDelegateMsg`] carries the opposite trade-off, and is marked: its
/// consumers are third-party delegate WASM, which can reasonably ignore a
/// variant it does not know about.
///
/// # Wire format and compatibility
///
/// bincode, variant index 0..=N in **declaration order**. Never insert or
/// reorder a variant: that silently reassigns every later tag, and deployed
/// delegate WASM built against an older stdlib would encode into what the host
/// now reads as a different variant. `delegate_msg_variant_tags_are_pinned`
/// pins every tag so a reorder fails CI rather than production.
///
/// Appending is compatible in one direction only, and this enum is the
/// direction that bites:
///
/// - **Old delegate → new host: fine, for appended VARIANTS.** The host
///   understands every tag an older delegate can emit, so deployed delegate
///   WASM keeps working against an upgraded node with no rebuild. This does
///   **not** extend to appending a FIELD to an existing variant's payload
///   struct, because a field breaks in the opposite direction. See
///   `struct_field_wire_compat` in `client_api::client_events`.
///   (`ApplicationMessage` is `#[non_exhaustive]`, which invites precisely that
///   edit. It is the only payload struct here that is.)
/// - **New delegate → old host: fails, and fails loudly.** bincode rejects the
///   unknown variant tag — as `ErrorKind::Custom("invalid value: integer `N`,
///   expected variant index 0 <= i < M")`, since it hands the index to serde's
///   derived visitor rather than validating it itself — so the host surfaces a
///   decode error on that message rather than misreading it.
///
/// There is deliberately **no feature-detection handshake**. A delegate cannot
/// ask the host which variants it understands, and adding a probe would itself
/// be a wire change with the same bootstrapping problem. The rule is therefore
/// the blunt one: **a delegate that emits a variant introduced in stdlib
/// version X requires a host built against stdlib >= X.**
///
/// A delegate that must work against older hosts has one good alternative: the
/// V2 host-function API (the `freenet_delegate_contracts` import namespace).
/// Host functions are resolved **by name at module instantiation**, so an
/// import an old host does not provide fails at load time with a named
/// missing-import error, instead of mid-protocol on a decode. That is the
/// better failure mode, and it is why new capabilities should prefer a host
/// function over a new variant where there is a choice.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OutboundDelegateMsg {
    // for the apps
    ApplicationMessage(ApplicationMessage),
    RequestUserInput(
        #[serde(deserialize_with = "OutboundDelegateMsg::deser_user_input_req")]
        UserInputRequest<'static>,
    ),
    // todo: remove when context can be accessed from the delegate environment and we pass it as reference
    ContextUpdated(DelegateContext),
    GetContractRequest(GetContractRequest),
    PutContractRequest(PutContractRequest),
    UpdateContractRequest(UpdateContractRequest),
    SubscribeContractRequest(SubscribeContractRequest),
    SendDelegateMessage(DelegateMessage),
    // Appended in 0.9.0 at tag 8. New variants go at the END, never inserted —
    // see the wire-format note on this enum. freenet-stdlib#82 also appends
    // here (ScheduleWakeup) and must therefore move to tag 9; at the time of
    // writing that PR still declares tag 8, so whichever lands second will trip
    // the pin, which is the intended outcome rather than a surprise.
    UnsubscribeContractRequest(UnsubscribeContractRequest),
}

impl From<ApplicationMessage> for OutboundDelegateMsg {
    fn from(req: ApplicationMessage) -> Self {
        Self::ApplicationMessage(req)
    }
}

impl From<GetContractRequest> for OutboundDelegateMsg {
    fn from(req: GetContractRequest) -> Self {
        Self::GetContractRequest(req)
    }
}

impl From<PutContractRequest> for OutboundDelegateMsg {
    fn from(req: PutContractRequest) -> Self {
        Self::PutContractRequest(req)
    }
}

impl From<UpdateContractRequest> for OutboundDelegateMsg {
    fn from(req: UpdateContractRequest) -> Self {
        Self::UpdateContractRequest(req)
    }
}

impl From<SubscribeContractRequest> for OutboundDelegateMsg {
    fn from(req: SubscribeContractRequest) -> Self {
        Self::SubscribeContractRequest(req)
    }
}

impl From<UnsubscribeContractRequest> for OutboundDelegateMsg {
    fn from(req: UnsubscribeContractRequest) -> Self {
        Self::UnsubscribeContractRequest(req)
    }
}

impl From<DelegateMessage> for OutboundDelegateMsg {
    fn from(msg: DelegateMessage) -> Self {
        Self::SendDelegateMessage(msg)
    }
}

impl OutboundDelegateMsg {
    fn deser_user_input_req<'de, D>(deser: D) -> Result<UserInputRequest<'static>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <UserInputRequest<'de> as Deserialize>::deserialize(deser)?;
        Ok(value.into_owned())
    }

    pub fn processed(&self) -> bool {
        match self {
            OutboundDelegateMsg::ApplicationMessage(msg) => msg.processed,
            OutboundDelegateMsg::GetContractRequest(msg) => msg.processed,
            OutboundDelegateMsg::PutContractRequest(msg) => msg.processed,
            OutboundDelegateMsg::UpdateContractRequest(msg) => msg.processed,
            OutboundDelegateMsg::SubscribeContractRequest(msg) => msg.processed,
            OutboundDelegateMsg::UnsubscribeContractRequest(msg) => msg.processed,
            OutboundDelegateMsg::SendDelegateMessage(msg) => msg.processed,
            OutboundDelegateMsg::RequestUserInput(_) => true,
            OutboundDelegateMsg::ContextUpdated(_) => true,
        }
    }

    pub fn get_context(&self) -> Option<&DelegateContext> {
        match self {
            OutboundDelegateMsg::ApplicationMessage(ApplicationMessage { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::GetContractRequest(GetContractRequest { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::PutContractRequest(PutContractRequest { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::UpdateContractRequest(UpdateContractRequest {
                context, ..
            }) => Some(context),
            OutboundDelegateMsg::SubscribeContractRequest(SubscribeContractRequest {
                context,
                ..
            }) => Some(context),
            OutboundDelegateMsg::UnsubscribeContractRequest(UnsubscribeContractRequest {
                context,
                ..
            }) => Some(context),
            OutboundDelegateMsg::SendDelegateMessage(DelegateMessage { context, .. }) => {
                Some(context)
            }
            _ => None,
        }
    }

    pub fn get_mut_context(&mut self) -> Option<&mut DelegateContext> {
        match self {
            OutboundDelegateMsg::ApplicationMessage(ApplicationMessage { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::GetContractRequest(GetContractRequest { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::PutContractRequest(PutContractRequest { context, .. }) => {
                Some(context)
            }
            OutboundDelegateMsg::UpdateContractRequest(UpdateContractRequest {
                context, ..
            }) => Some(context),
            OutboundDelegateMsg::SubscribeContractRequest(SubscribeContractRequest {
                context,
                ..
            }) => Some(context),
            OutboundDelegateMsg::UnsubscribeContractRequest(UnsubscribeContractRequest {
                context,
                ..
            }) => Some(context),
            OutboundDelegateMsg::SendDelegateMessage(DelegateMessage { context, .. }) => {
                Some(context)
            }
            _ => None,
        }
    }
}

/// Request to get contract state from within a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetContractRequest {
    pub contract_id: ContractInstanceId,
    pub context: DelegateContext,
    pub processed: bool,
}

impl GetContractRequest {
    pub fn new(contract_id: ContractInstanceId) -> Self {
        Self {
            contract_id,
            context: Default::default(),
            processed: false,
        }
    }
}

/// Response containing contract state for a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetContractResponse {
    pub contract_id: ContractInstanceId,
    /// The contract state, or None if the contract was not found locally.
    pub state: Option<WrappedState>,
    pub context: DelegateContext,
}

/// Request to store a new contract from within a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutContractRequest {
    /// The contract code and parameters.
    pub contract: ContractContainer,
    /// The initial state for the contract.
    pub state: WrappedState,
    /// Related contracts that this contract depends on.
    #[serde(deserialize_with = "RelatedContracts::deser_related_contracts")]
    pub related_contracts: RelatedContracts<'static>,
    /// Context for the delegate.
    pub context: DelegateContext,
    /// Whether this request has been processed.
    pub processed: bool,
}

impl PutContractRequest {
    pub fn new(
        contract: ContractContainer,
        state: WrappedState,
        related_contracts: RelatedContracts<'static>,
    ) -> Self {
        Self {
            contract,
            state,
            related_contracts,
            context: Default::default(),
            processed: false,
        }
    }
}

/// Response after attempting to store a contract from a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutContractResponse {
    /// The ID of the contract that was (attempted to be) stored.
    pub contract_id: ContractInstanceId,
    /// Success (Ok) or error message (Err).
    pub result: Result<(), String>,
    /// Context for the delegate.
    pub context: DelegateContext,
}

/// Request to update an existing contract's state from within a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateContractRequest {
    /// The contract to update.
    pub contract_id: ContractInstanceId,
    /// The update to apply (full state or delta).
    #[serde(deserialize_with = "UpdateContractRequest::deser_update_data")]
    pub update: UpdateData<'static>,
    /// Context for the delegate.
    pub context: DelegateContext,
    /// Whether this request has been processed.
    pub processed: bool,
}

impl UpdateContractRequest {
    pub fn new(contract_id: ContractInstanceId, update: UpdateData<'static>) -> Self {
        Self {
            contract_id,
            update,
            context: Default::default(),
            processed: false,
        }
    }

    fn deser_update_data<'de, D>(deser: D) -> Result<UpdateData<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = <UpdateData<'de> as Deserialize>::deserialize(deser)?;
        Ok(value.into_owned())
    }
}

/// Response after attempting to update a contract from a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateContractResponse {
    /// The contract that was updated.
    pub contract_id: ContractInstanceId,
    /// Success (Ok) or error message (Err).
    pub result: Result<(), String>,
    /// Context for the delegate.
    pub context: DelegateContext,
}

/// Request to subscribe to a contract's state changes from within a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribeContractRequest {
    /// The contract to subscribe to.
    pub contract_id: ContractInstanceId,
    /// Context for the delegate.
    pub context: DelegateContext,
    /// Whether this request has been processed.
    pub processed: bool,
}

impl SubscribeContractRequest {
    pub fn new(contract_id: ContractInstanceId) -> Self {
        Self {
            contract_id,
            context: Default::default(),
            processed: false,
        }
    }
}

/// Response after attempting to subscribe to a contract from a delegate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribeContractResponse {
    /// The contract subscribed to.
    pub contract_id: ContractInstanceId,
    /// Success (Ok) or error message (Err).
    pub result: Result<(), String>,
    /// Context for the delegate.
    pub context: DelegateContext,
}

/// Request to stop receiving a contract's state changes, from within a delegate.
///
/// The counterpart of [`SubscribeContractRequest`]. Before 0.9.0 a delegate had
/// no way to drop a subscription it had taken: the only release path was the
/// implicit cleanup when the delegate itself was unregistered, so a delegate
/// that had finished with a contract went on holding interest in it for as long
/// as the delegate existed. Specified in freenet-core#2830 alongside subscribe;
/// only subscribe was built.
///
/// Answered with [`InboundDelegateMsg::UnsubscribeContractResponse`].
///
/// Field order is the wire format. Do not reorder.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnsubscribeContractRequest {
    /// The contract to stop receiving notifications for.
    pub contract_id: ContractInstanceId,
    /// Context for the delegate.
    pub context: DelegateContext,
    /// Whether this request has been processed.
    pub processed: bool,
}

impl UnsubscribeContractRequest {
    pub fn new(contract_id: ContractInstanceId) -> Self {
        Self {
            contract_id,
            context: Default::default(),
            processed: false,
        }
    }
}

/// Response after attempting to unsubscribe from a contract from a delegate.
///
/// **Unsubscribing a contract the delegate is not subscribed to reports
/// `Ok(())`, not an error.** That is not a convenience: it is what the host
/// actually does. Teardown goes through the same removal path that a
/// no-longer-present client id already takes as a no-op, so returning an error
/// would have the host inventing a failure it did not have. It also matches the
/// subscribe side, where a repeat subscribe is a set insert.
///
/// Field order is the wire format. Do not reorder.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnsubscribeContractResponse {
    /// The contract unsubscribed from.
    pub contract_id: ContractInstanceId,
    /// Success (Ok) or error message (Err). Unsubscribing a contract the
    /// delegate was not subscribed to reports `Ok(())`.
    pub result: Result<(), String>,
    /// Context for the delegate.
    pub context: DelegateContext,
}

/// A message sent from one delegate to another.
///
/// Delegates can communicate with each other by emitting
/// `OutboundDelegateMsg::SendDelegateMessage` with a `DelegateMessage` targeting
/// another delegate. The runtime delivers it as `InboundDelegateMsg::DelegateMessage`
/// to the target delegate's `process()` function.
///
/// The `sender` field is overwritten by the runtime with the actual sender's key
/// (sender attestation), so delegates cannot spoof their identity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegateMessage {
    /// The delegate to deliver this message to.
    pub target: DelegateKey,
    /// The delegate that sent this message (overwritten by runtime for attestation).
    pub sender: DelegateKey,
    /// Arbitrary message payload.
    pub payload: Vec<u8>,
    /// Delegate context, carried through the processing pipeline.
    pub context: DelegateContext,
    /// Runtime protocol flag indicating whether this message has been delivered.
    pub processed: bool,
}

impl DelegateMessage {
    pub fn new(target: DelegateKey, sender: DelegateKey, payload: Vec<u8>) -> Self {
        Self {
            target,
            sender,
            payload,
            context: DelegateContext::default(),
            processed: false,
        }
    }
}

/// Notification delivered to a delegate when a subscribed contract's state changes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractNotification {
    /// The contract whose state changed.
    pub contract_id: ContractInstanceId,
    /// The new state of the contract.
    pub new_state: WrappedState,
    /// Context for the delegate.
    pub context: DelegateContext,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NotificationMessage<'a>(
    #[serde_as(as = "serde_with::Bytes")]
    #[serde(borrow)]
    Cow<'a, [u8]>,
);

impl TryFrom<&serde_json::Value> for NotificationMessage<'static> {
    type Error = ();

    fn try_from(json: &serde_json::Value) -> Result<NotificationMessage<'static>, ()> {
        // todo: validate format when we have a better idea of what we want here
        let bytes = serde_json::to_vec(json).unwrap();
        Ok(Self(Cow::Owned(bytes)))
    }
}

impl NotificationMessage<'_> {
    pub fn into_owned(self) -> NotificationMessage<'static> {
        NotificationMessage(self.0.into_owned().into())
    }
    pub fn bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientResponse<'a>(
    #[serde_as(as = "serde_with::Bytes")]
    #[serde(borrow)]
    Cow<'a, [u8]>,
);

impl Deref for ClientResponse<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ClientResponse<'_> {
    pub fn new(response: Vec<u8>) -> Self {
        Self(response.into())
    }
    pub fn into_owned(self) -> ClientResponse<'static> {
        ClientResponse(self.0.into_owned().into())
    }
    pub fn bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInputRequest<'a> {
    pub request_id: u32,
    #[serde(borrow)]
    /// An interpretable message by the notification system.
    pub message: NotificationMessage<'a>,
    /// If a response is required from the user they can be chosen from this list.
    pub responses: Vec<ClientResponse<'a>>,
}

impl UserInputRequest<'_> {
    pub fn into_owned(self) -> UserInputRequest<'static> {
        UserInputRequest {
            request_id: self.request_id,
            message: self.message.into_owned(),
            responses: self.responses.into_iter().map(|r| r.into_owned()).collect(),
        }
    }
}

#[doc(hidden)]
pub(crate) mod wasm_interface {
    //! Contains all the types to interface between the host environment and
    //! the wasm module execution.
    use super::*;
    use crate::memory::WasmLinearMem;

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct DelegateInterfaceResult {
        ptr: i64,
        size: u32,
    }

    impl DelegateInterfaceResult {
        pub unsafe fn from_raw(ptr: i64, mem: &WasmLinearMem) -> Self {
            let result = Box::leak(Box::from_raw(crate::memory::buf::compute_ptr(
                ptr as *mut Self,
                mem,
            )));
            #[cfg(feature = "trace")]
            {
                tracing::trace!(
                    "got FFI result @ {ptr} ({:p}) -> {result:?}",
                    ptr as *mut Self
                );
            }
            *result
        }

        #[cfg(feature = "contract")]
        pub fn into_raw(self) -> i64 {
            #[cfg(feature = "trace")]
            {
                tracing::trace!("returning FFI -> {self:?}");
            }
            let ptr = Box::into_raw(Box::new(self));
            #[cfg(feature = "trace")]
            {
                tracing::trace!("FFI result ptr: {ptr:p} ({}i64)", ptr as i64);
            }
            ptr as _
        }

        pub unsafe fn unwrap(
            self,
            mem: WasmLinearMem,
        ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
            let ptr = crate::memory::buf::compute_ptr(self.ptr as *mut u8, &mem);
            let serialized = std::slice::from_raw_parts(ptr as *const u8, self.size as _);
            let value: Result<Vec<OutboundDelegateMsg>, DelegateError> =
                bincode::deserialize(serialized)
                    .map_err(|e| DelegateError::Other(format!("{e}")))?;
            #[cfg(feature = "trace")]
            {
                tracing::trace!(
                    "got result through FFI; addr: {:p} ({}i64, mapped: {ptr:p})
                     serialized: {serialized:?}
                     value: {value:?}",
                    self.ptr as *mut u8,
                    self.ptr
                );
            }
            value
        }
    }

    impl From<Result<Vec<OutboundDelegateMsg>, DelegateError>> for DelegateInterfaceResult {
        fn from(value: Result<Vec<OutboundDelegateMsg>, DelegateError>) -> Self {
            let serialized = bincode::serialize(&value).unwrap();
            let size = serialized.len() as _;
            let ptr = serialized.as_ptr();
            #[cfg(feature = "trace")]
            {
                tracing::trace!(
                    "sending result through FFI; addr: {ptr:p} ({}),\n  serialized: {serialized:?}\n  value: {value:?}",
                    ptr as i64
                );
            }
            std::mem::forget(serialized);
            Self {
                ptr: ptr as i64,
                size,
            }
        }
    }
}

#[cfg(test)]
mod message_origin_tests {
    use super::*;

    /// Wire-format pin: bincode encoding of `MessageOrigin::WebApp(..)` must
    /// stay byte-identical across stdlib releases. Deployed delegate WASM
    /// compiled against an older stdlib will receive these bytes from a
    /// host running the new stdlib and must continue to deserialize them.
    /// If this test ever fails, it is a wire-format break and is NOT
    /// publishable as a non-major bump.
    #[test]
    fn webapp_origin_wire_format_is_stable() {
        let id = ContractInstanceId::new([0xABu8; 32]);
        let origin = MessageOrigin::WebApp(id);
        let encoded = bincode::serialize(&origin).unwrap();

        // Variant tag 0 (4-byte LE u32 in default bincode config) followed by
        // the 32 raw bytes of the ContractInstanceId.
        let mut expected = vec![0u8, 0, 0, 0];
        expected.extend_from_slice(&[0xABu8; 32]);
        assert_eq!(encoded, expected);
    }

    /// Wire-format pin for the `Delegate` variant. Locks the full byte
    /// layout (variant tag + serde repr of `DelegateKey`) so that any future
    /// change to either `DelegateKey`'s serde or the workspace bincode
    /// config is caught loudly. If `DelegateKey`'s on-the-wire encoding
    /// changes, deployed delegates compiled against a previous stdlib will
    /// silently fail to deserialize inter-delegate origins — which is
    /// exactly the failure mode this test exists to prevent.
    #[test]
    fn delegate_origin_wire_format_is_stable() {
        let key = DelegateKey::new([0x11u8; 32], crate::code_hash::CodeHash::new([0x22u8; 32]));
        let origin = MessageOrigin::Delegate(key);
        let encoded = bincode::serialize(&origin).unwrap();

        // Variant tag 1 (4-byte LE u32 in default bincode config), followed
        // by the 32-byte `key` field, followed by the 32-byte `code_hash`
        // field of `DelegateKey`.
        let mut expected = vec![1u8, 0, 0, 0];
        expected.extend_from_slice(&[0x11u8; 32]);
        expected.extend_from_slice(&[0x22u8; 32]);
        assert_eq!(encoded, expected);

        // And it must still round-trip.
        let decoded: MessageOrigin = bincode::deserialize(&encoded).unwrap();
        assert!(matches!(decoded, MessageOrigin::Delegate(_)));
    }

    /// Wire-format pin for the first variant of [`InboundDelegateMsg`]. Pins
    /// the tag so that reordering the enum cannot silently shift existing
    /// deployed delegate WASM off the correct variant. Only tag+payload
    /// prefix is asserted (not the full ApplicationMessage byte layout),
    /// since ApplicationMessage's internal fields have their own stability
    /// expectations handled at a different layer. What matters here is that
    /// variant 0 stays `ApplicationMessage` on the wire.
    #[test]
    fn inbound_delegate_msg_wire_format_is_stable() {
        let msg = InboundDelegateMsg::ApplicationMessage(ApplicationMessage::new(vec![0xCC]));
        let encoded = bincode::serialize(&msg).unwrap();
        assert_eq!(
            encoded[..4],
            [0, 0, 0, 0],
            "ApplicationMessage must stay at variant tag 0 on the wire; \
             reordering InboundDelegateMsg variants is a wire-format break"
        );
        // And it must still round-trip into the same variant.
        let decoded: InboundDelegateMsg<'_> = bincode::deserialize(&encoded).unwrap();
        assert!(matches!(decoded, InboundDelegateMsg::ApplicationMessage(_)));
    }
}

/// Executable evidence for the wire-compatibility rules documented on
/// [`InboundDelegateMsg`] and [`OutboundDelegateMsg`].
///
/// The claims those doc comments make about bincode's behaviour are asserted
/// here rather than believed, because every one of them is the kind of claim
/// that is easy to state, easy to get backwards, and impossible to notice being
/// wrong until deployed delegate WASM misreads a message in production.
#[cfg(test)]
mod delegate_wire_compat {
    use super::*;
    use crate::contract_interface::WrappedContract;
    use crate::prelude::ContractCode;
    use crate::versioning::ContractWasmAPIVersion;
    use std::sync::Arc;

    /// The number of variants each enum has **today**. These are not free
    /// parameters: see `an_unpinned_variant_fails_this_test`, which is what
    /// makes them fail closed rather than drift.
    const INBOUND_VARIANT_COUNT: u32 = 9;
    const OUTBOUND_VARIANT_COUNT: u32 = 9;

    fn instance_id() -> ContractInstanceId {
        ContractInstanceId::new([0x5Au8; 32])
    }

    fn delegate_key() -> DelegateKey {
        DelegateKey::new([0x11u8; 32], CodeHash::new([0x22u8; 32]))
    }

    fn contract_container() -> ContractContainer {
        ContractContainer::Wasm(ContractWasmAPIVersion::V1(WrappedContract::new(
            Arc::new(ContractCode::from(vec![1u8, 2, 3])),
            Parameters::from(vec![9u8, 8, 7]),
        )))
    }

    /// The bincode variant tag actually on the wire: a 4-byte little-endian
    /// u32 prefix (this workspace's bincode config uses fixint encoding).
    fn wire_tag(encoded: &[u8]) -> u32 {
        u32::from_le_bytes(
            encoded[..4]
                .try_into()
                .expect("a bincode enum encoding starts with a 4-byte tag"),
        )
    }

    /// The tag each [`InboundDelegateMsg`] variant is frozen at, forever.
    ///
    /// This match is **exhaustive on purpose**. `#[non_exhaustive]` has no
    /// effect inside the crate that defines the enum, so adding a variant
    /// without adding an arm here is a **compile error** — which is the point.
    /// A new variant cannot slip in unpinned.
    ///
    /// If you are here because you added a variant: give it the next unused
    /// number, append it at the END of the enum, add it to `every_inbound`
    /// below, and bump `INBOUND_VARIANT_COUNT`. Do not renumber anything.
    fn pinned_inbound_tag(msg: &InboundDelegateMsg<'_>) -> u32 {
        match msg {
            InboundDelegateMsg::ApplicationMessage(_) => 0,
            InboundDelegateMsg::UserResponse(_) => 1,
            InboundDelegateMsg::GetContractResponse(_) => 2,
            InboundDelegateMsg::PutContractResponse(_) => 3,
            InboundDelegateMsg::UpdateContractResponse(_) => 4,
            InboundDelegateMsg::SubscribeContractResponse(_) => 5,
            InboundDelegateMsg::ContractNotification(_) => 6,
            InboundDelegateMsg::DelegateMessage(_) => 7,
            InboundDelegateMsg::UnsubscribeContractResponse(_) => 8,
        }
    }

    /// The tag each [`OutboundDelegateMsg`] variant is frozen at, forever.
    /// Exhaustive for the same reason as [`pinned_inbound_tag`].
    fn pinned_outbound_tag(msg: &OutboundDelegateMsg) -> u32 {
        match msg {
            OutboundDelegateMsg::ApplicationMessage(_) => 0,
            OutboundDelegateMsg::RequestUserInput(_) => 1,
            OutboundDelegateMsg::ContextUpdated(_) => 2,
            OutboundDelegateMsg::GetContractRequest(_) => 3,
            OutboundDelegateMsg::PutContractRequest(_) => 4,
            OutboundDelegateMsg::UpdateContractRequest(_) => 5,
            OutboundDelegateMsg::SubscribeContractRequest(_) => 6,
            OutboundDelegateMsg::SendDelegateMessage(_) => 7,
            OutboundDelegateMsg::UnsubscribeContractRequest(_) => 8,
        }
    }

    /// One value of every [`InboundDelegateMsg`] variant.
    fn every_inbound() -> Vec<InboundDelegateMsg<'static>> {
        let id = instance_id();
        let ctx = DelegateContext::default();
        vec![
            InboundDelegateMsg::ApplicationMessage(ApplicationMessage::new(vec![0xCC])),
            InboundDelegateMsg::UserResponse(UserInputResponse {
                request_id: 7,
                response: ClientResponse::new(vec![0x01]),
                context: ctx.clone(),
            }),
            InboundDelegateMsg::GetContractResponse(GetContractResponse {
                contract_id: id,
                state: None,
                context: ctx.clone(),
            }),
            InboundDelegateMsg::PutContractResponse(PutContractResponse {
                contract_id: id,
                result: Ok(()),
                context: ctx.clone(),
            }),
            InboundDelegateMsg::UpdateContractResponse(UpdateContractResponse {
                contract_id: id,
                result: Ok(()),
                context: ctx.clone(),
            }),
            InboundDelegateMsg::SubscribeContractResponse(SubscribeContractResponse {
                contract_id: id,
                result: Ok(()),
                context: ctx.clone(),
            }),
            InboundDelegateMsg::ContractNotification(ContractNotification {
                contract_id: id,
                new_state: WrappedState::new(vec![0xAB]),
                context: ctx.clone(),
            }),
            InboundDelegateMsg::DelegateMessage(DelegateMessage::new(
                delegate_key(),
                delegate_key(),
                vec![0xEE],
            )),
            InboundDelegateMsg::UnsubscribeContractResponse(UnsubscribeContractResponse {
                contract_id: id,
                result: Ok(()),
                context: ctx.clone(),
            }),
        ]
    }

    /// One value of every [`OutboundDelegateMsg`] variant.
    ///
    /// Every variant is covered, `PutContractRequest` included: building a
    /// `ContractContainer` is four lines (see `contract_container`), and a pin
    /// test with a hole in it is exactly the shape of guard that reads as
    /// coverage while providing none.
    fn every_outbound() -> Vec<OutboundDelegateMsg> {
        let id = instance_id();
        vec![
            OutboundDelegateMsg::ApplicationMessage(ApplicationMessage::new(vec![0xCC])),
            OutboundDelegateMsg::RequestUserInput(UserInputRequest {
                request_id: 7,
                message: NotificationMessage(Cow::Owned(vec![0x02])),
                responses: vec![],
            }),
            OutboundDelegateMsg::ContextUpdated(DelegateContext::default()),
            OutboundDelegateMsg::GetContractRequest(GetContractRequest::new(id)),
            OutboundDelegateMsg::PutContractRequest(PutContractRequest::new(
                contract_container(),
                WrappedState::new(vec![0xAB]),
                RelatedContracts::default(),
            )),
            OutboundDelegateMsg::UpdateContractRequest(UpdateContractRequest::new(
                id,
                UpdateData::State(vec![0xAB].into()),
            )),
            OutboundDelegateMsg::SubscribeContractRequest(SubscribeContractRequest::new(id)),
            OutboundDelegateMsg::SendDelegateMessage(DelegateMessage::new(
                delegate_key(),
                delegate_key(),
                vec![0xEE],
            )),
            OutboundDelegateMsg::UnsubscribeContractRequest(UnsubscribeContractRequest::new(id)),
        ]
    }

    /// Pins the bincode variant tag of **every** variant of both delegate
    /// message enums.
    ///
    /// The pin this replaces covered `InboundDelegateMsg`'s variant 0 alone, so
    /// any reorder that happened to leave `ApplicationMessage` first — swapping
    /// `UserResponse` and `GetContractResponse`, say — went undetected. That is
    /// not a theoretical gap: exactly that swap was written, and staged, during
    /// the work that produced this test.
    ///
    /// A reorder is the dangerous edit precisely because it is silent. The
    /// bytes still decode. They decode into the wrong variant, and the failure
    /// surfaces as a delegate acting on a message it was never sent.
    ///
    /// **If this test fails, do not update the expected numbers.** Either a
    /// variant was inserted or reordered (revert it; append instead), or one
    /// was removed — which reassigns every later tag and is a wire break
    /// needing a deliberate release decision. See the
    /// `RegisterDelegateWithPredecessors` removal in 0.9.0 for the shape of
    /// that decision: it was appended last specifically so that removing it
    /// renumbered nothing.
    #[test]
    fn delegate_msg_variant_tags_are_pinned() {
        for msg in every_inbound() {
            let expected = pinned_inbound_tag(&msg);
            let encoded = bincode::serialize(&msg).expect("inbound must serialize");
            assert_eq!(
                wire_tag(&encoded),
                expected,
                "InboundDelegateMsg::{msg:?} moved off wire tag {expected}; inserting, \
                 reordering or removing variants breaks deployed delegate WASM"
            );
        }

        for msg in every_outbound() {
            let expected = pinned_outbound_tag(&msg);
            let encoded = bincode::serialize(&msg).expect("outbound must serialize");
            assert_eq!(
                wire_tag(&encoded),
                expected,
                "OutboundDelegateMsg::{msg:?} moved off wire tag {expected}; inserting, \
                 reordering or removing variants breaks deployed delegate WASM"
            );
        }
    }

    /// Every variant is actually exercised by the pin above.
    ///
    /// [`pinned_inbound_tag`] is exhaustive, so a new variant cannot be left
    /// unpinned without a compile error — but it *could* be left out of
    /// `every_inbound`, and then the pin would silently stop covering it.
    /// Asserting that the sampled tags are exactly `0..COUNT`, with no gaps and
    /// no repeats, closes that.
    #[test]
    fn every_variant_is_covered_by_the_pin() {
        let mut inbound: Vec<u32> = every_inbound().iter().map(pinned_inbound_tag).collect();
        inbound.sort_unstable();
        assert_eq!(
            inbound,
            (0..INBOUND_VARIANT_COUNT).collect::<Vec<_>>(),
            "every_inbound must contain each InboundDelegateMsg variant exactly once"
        );

        let mut outbound: Vec<u32> = every_outbound().iter().map(pinned_outbound_tag).collect();
        outbound.sort_unstable();
        assert_eq!(
            outbound,
            (0..OUTBOUND_VARIANT_COUNT).collect::<Vec<_>>(),
            "every_outbound must contain each OutboundDelegateMsg variant exactly once"
        );
    }

    /// The count constants above cannot be allowed to drift, so this probes the
    /// enums themselves: a payload whose tag is one past the last known variant
    /// must fail to decode.
    ///
    /// This is the test that fails **closed**. Add a variant and forget
    /// everything else here, and the tag that was previously undecodable
    /// becomes decodable, and this fails. Without it, `INBOUND_VARIANT_COUNT`
    /// would be a number asserted only against a list written by the same hand
    /// in the same commit — which is not a check, it is a restatement.
    ///
    /// The payload is a run of zero bytes after the tag, which decodes as
    /// empty vectors, `None`, `Ok`, `false` and zeroed arrays, so it satisfies
    /// essentially any variant shape a new variant is likely to have. Trailing
    /// bytes are ignored: `bincode::deserialize` configures
    /// `allow_trailing_bytes()` (bincode-1.3.3 `src/lib.rs`), which is also why
    /// a fixed-size probe is safe here.
    #[test]
    fn an_unpinned_variant_fails_this_test() {
        // The probe must fail because the TAG is unknown, not because a
        // payload of zeros happened not to parse. Asserting only `is_err()`
        // would let a new variant whose first field rejects zeros (a
        // `DateTime`, a `NonZero*`, a validating `deserialize_with`) go
        // undetected: the tag would be valid, the decode would still fail, and
        // this test would stay green while the counts drifted.
        //
        // bincode hands an out-of-range variant index to serde's derived
        // visitor, which rejects it as `invalid value: integer `N`, expected
        // variant index 0 <= i < M` — an `ErrorKind::Custom`. Match on that
        // wording rather than on `InvalidTagEncoding`, which bincode produces
        // only for a bad `Option` discriminant.
        fn assert_rejected_as_unknown_variant(err: &bincode::Error, tag: u32, which: &str) {
            let msg = err.to_string();
            assert!(
                msg.contains("variant index"),
                "tag {tag} on {which} failed for the wrong reason ({msg}); the tag itself must \
                 still be unknown, otherwise a variant was added without updating the count, \
                 the pinned_*_tag match and the every_* list"
            );
        }

        let mut probe = INBOUND_VARIANT_COUNT.to_le_bytes().to_vec();
        probe.extend_from_slice(&[0u8; 256]);
        let err = match bincode::deserialize::<InboundDelegateMsg<'_>>(&probe) {
            Ok(v) => panic!(
                "tag {INBOUND_VARIANT_COUNT} must not decode as an InboundDelegateMsg, got {v:?}"
            ),
            Err(e) => e,
        };
        assert_rejected_as_unknown_variant(&err, INBOUND_VARIANT_COUNT, "InboundDelegateMsg");

        let mut probe = OUTBOUND_VARIANT_COUNT.to_le_bytes().to_vec();
        probe.extend_from_slice(&[0u8; 256]);
        let err = match bincode::deserialize::<OutboundDelegateMsg>(&probe) {
            Ok(v) => panic!(
                "tag {OUTBOUND_VARIANT_COUNT} must not decode as an OutboundDelegateMsg, got {v:?}"
            ),
            Err(e) => e,
        };
        assert_rejected_as_unknown_variant(&err, OUTBOUND_VARIANT_COUNT, "OutboundDelegateMsg");

        // Control, so the probe cannot pass vacuously from the other end: the
        // LAST known tag must still decode from the same all-zero payload. If
        // this ever fails, the zero payload has stopped being a valid encoding
        // for the final variant, and the probes above are no longer testing
        // what they claim.
        let mut control = (INBOUND_VARIANT_COUNT - 1).to_le_bytes().to_vec();
        control.extend_from_slice(&[0u8; 256]);
        bincode::deserialize::<InboundDelegateMsg<'_>>(&control).expect(
            "the LAST inbound variant's payload must be decodable from zeros, or this probe can \
             no longer tell an unknown tag from an unparseable payload. If a variant whose \
             payload rejects zeros was just appended, do not delete this — point the control at \
             a variant that still decodes from zeros",
        );

        let mut control = (OUTBOUND_VARIANT_COUNT - 1).to_le_bytes().to_vec();
        control.extend_from_slice(&[0u8; 256]);
        bincode::deserialize::<OutboundDelegateMsg>(&control).expect(
            "the LAST outbound variant's payload must be decodable from zeros — see the inbound \
             control above for what to do if that stops being true",
        );
    }

    /// Direction 1 of the append rule: **old sender to new receiver works.**
    ///
    /// The payload is hand-built rather than produced by this crate's own
    /// encoder, so it stands in for bytes emitted by a delegate compiled
    /// against an older stdlib; an encoder-produced value would only prove the
    /// code agrees with itself.
    ///
    /// Named for what it actually pins. Nothing here appends a variant — the
    /// test cannot fail *because of* an append, only because a tag moved or a
    /// payload layout changed, which `delegate_msg_variant_tags_are_pinned`
    /// also covers. Its distinct value is that the expected bytes are written
    /// out by hand, so a change to `ContractNotification`'s field order or to
    /// the bincode config fails here with a concrete byte string to compare
    /// against. Direction 2, which genuinely models an old receiver, is
    /// `a_new_variant_does_not_decode_on_an_old_receiver` below.
    #[test]
    fn a_hand_built_old_encoder_payload_decodes_into_the_same_variant() {
        // InboundDelegateMsg tag 6 = ContractNotification { contract_id,
        // new_state: WrappedState (empty), context: DelegateContext (empty) }.
        let mut old_payload = vec![6u8, 0, 0, 0];
        old_payload.extend_from_slice(&[0x5Au8; 32]);
        old_payload.extend_from_slice(&0u64.to_le_bytes()); // new_state: len 0
        old_payload.extend_from_slice(&0u64.to_le_bytes()); // context: len 0

        let decoded: InboundDelegateMsg<'_> = bincode::deserialize(&old_payload)
            .expect("a payload predating any appended variant must still decode");
        match decoded {
            InboundDelegateMsg::ContractNotification(n) => {
                assert_eq!(n.contract_id, instance_id());
            }
            other => panic!("an old ContractNotification decoded as {other:?}"),
        }
    }

    /// Direction 2 of the append rule: **new sender to old receiver fails, and
    /// fails loudly.** This is the direction the docs warn about, so it is
    /// asserted rather than assumed.
    ///
    /// An old receiver is modelled by an enum with a truncated tag space,
    /// which is exactly what an older stdlib's version of these types is. The
    /// point is that the failure is an `Err` — not a silent mis-decode into
    /// whatever variant happens to sit at that index.
    #[test]
    fn a_new_variant_does_not_decode_on_an_old_receiver() {
        // An "old" OutboundDelegateMsg that knows tags 0..=6 only, i.e. one
        // built before `SendDelegateMessage` was appended at 7.
        // Variants are only ever produced by deserialization, never
        // constructed here — which is the whole point of the test.
        #[allow(dead_code)]
        #[derive(serde::Deserialize, Debug)]
        enum OldOutboundTagSpace {
            V0,
            V1,
            V2,
            V3,
            V4,
            V5,
            V6,
        }

        let new_msg = bincode::serialize(&OutboundDelegateMsg::SendDelegateMessage(
            DelegateMessage::new(delegate_key(), delegate_key(), vec![0xEE]),
        ))
        .expect("outbound must serialize");
        assert_eq!(wire_tag(&new_msg), 7);

        let decoded = bincode::deserialize::<OldOutboundTagSpace>(&new_msg);
        assert!(
            decoded.is_err(),
            "a receiver that predates a variant must REJECT it, not mis-decode it; \
             if this ever passes, the compatibility rule documented on \
             OutboundDelegateMsg is wrong and delegates are silently misreading messages"
        );
    }

    /// The unsubscribe pair added in 0.9.0 round-trips, and adding it did not
    /// disturb any payload that predates it.
    ///
    /// The pre-0.9.0 byte string is hand-built rather than produced by this
    /// crate, so it stands in for bytes from a delegate compiled before the
    /// pair existed. Both halves matter: the new variant must work, and the old
    /// ones must be untouched by its arrival.
    #[test]
    fn the_unsubscribe_pair_round_trips_and_disturbs_nothing_older() {
        let id = instance_id();

        let req =
            OutboundDelegateMsg::UnsubscribeContractRequest(UnsubscribeContractRequest::new(id));
        let encoded = bincode::serialize(&req).expect("request must serialize");
        assert_eq!(wire_tag(&encoded), 8, "unsubscribe request is frozen at 8");
        match bincode::deserialize::<OutboundDelegateMsg>(&encoded).expect("must round-trip") {
            OutboundDelegateMsg::UnsubscribeContractRequest(r) => {
                assert_eq!(r.contract_id, id);
                assert!(!r.processed);
            }
            other => panic!("round-tripped into {other:?}"),
        }

        let resp = InboundDelegateMsg::UnsubscribeContractResponse(UnsubscribeContractResponse {
            contract_id: id,
            result: Ok(()),
            context: DelegateContext::default(),
        });
        let encoded = bincode::serialize(&resp).expect("response must serialize");
        assert_eq!(wire_tag(&encoded), 8, "unsubscribe response is frozen at 8");
        match bincode::deserialize::<InboundDelegateMsg<'_>>(&encoded).expect("must round-trip") {
            InboundDelegateMsg::UnsubscribeContractResponse(r) => {
                // Assert the VALUES, not merely the variant. Checking only
                // `matches!` is what lets a field reorder through: the encoder
                // and decoder would still agree with each other.
                assert_eq!(r.contract_id, id);
                assert!(r.result.is_ok());
            }
            other => panic!("round-tripped into {other:?}"),
        }

        // Both structs' doc comments say the field ORDER is the wire format.
        // A round-trip through this crate's own encoder cannot establish that —
        // it proves the code agrees with itself, and a swap of `contract_id`
        // and `result` would round-trip just as happily. So the layout is
        // frozen as hand-written bytes, the same way ContractNotification is.
        let mut expected_resp = vec![8u8, 0, 0, 0];
        expected_resp.extend_from_slice(&[0x5Au8; 32]); // contract_id
        expected_resp.extend_from_slice(&0u32.to_le_bytes()); // result: Ok variant tag
        expected_resp.extend_from_slice(&0u64.to_le_bytes()); // context: empty
        assert_eq!(
            encoded, expected_resp,
            "UnsubscribeContractResponse layout is frozen: tag, contract_id, result, context"
        );

        let expected_req = {
            let mut v = vec![8u8, 0, 0, 0];
            v.extend_from_slice(&[0x5Au8; 32]); // contract_id
            v.extend_from_slice(&0u64.to_le_bytes()); // context: empty
            v.push(0u8); // processed: false
            v
        };
        assert_eq!(
            bincode::serialize(&req).expect("request must serialize"),
            expected_req,
            "UnsubscribeContractRequest layout is frozen: tag, contract_id, context, processed"
        );

        // The error path has a different bincode shape from Ok and is part of
        // the same frozen layout, so it is exercised rather than assumed.
        let err_resp =
            InboundDelegateMsg::UnsubscribeContractResponse(UnsubscribeContractResponse {
                contract_id: id,
                result: Err("nope".to_string()),
                context: DelegateContext::default(),
            });
        match bincode::deserialize::<InboundDelegateMsg<'_>>(
            &bincode::serialize(&err_resp).expect("must serialize"),
        )
        .expect("must round-trip")
        {
            InboundDelegateMsg::UnsubscribeContractResponse(r) => {
                assert_eq!(r.result.unwrap_err(), "nope");
            }
            other => panic!("error response round-tripped into {other:?}"),
        }

        // A ContractNotification encoded before 0.9.0 existed: tag 6, the 32
        // raw id bytes, an empty state and an empty context. Appending at 8
        // must leave it decoding exactly as it always did.
        let mut pre_0_9_0 = vec![6u8, 0, 0, 0];
        pre_0_9_0.extend_from_slice(&[0x5Au8; 32]);
        pre_0_9_0.extend_from_slice(&0u64.to_le_bytes());
        pre_0_9_0.extend_from_slice(&0u64.to_le_bytes());
        match bincode::deserialize::<InboundDelegateMsg<'_>>(&pre_0_9_0)
            .expect("a pre-0.9.0 payload must still decode")
        {
            InboundDelegateMsg::ContractNotification(n) => assert_eq!(n.contract_id, id),
            other => panic!("a pre-0.9.0 ContractNotification decoded as {other:?}"),
        }
    }

    /// Every inbound variant whose payload carries a `context` must return it.
    ///
    /// Both `get_context` and `get_mut_context` end in `_ => None`, so a
    /// missing arm is not a compile error — it silently reports "no context".
    /// That wildcard had already swallowed one: `UserResponse` carries a
    /// context and returned `None` for it, undetected, because nothing in the
    /// crate called either accessor.
    ///
    /// Driven off `every_inbound`, so a newly appended variant is covered the
    /// moment it is added to that list — which the tag pin already forces.
    #[test]
    fn every_inbound_variant_with_a_context_exposes_it() {
        for mut msg in every_inbound() {
            let carries_context = !matches!(msg, InboundDelegateMsg::ApplicationMessage(_));
            let tag = pinned_inbound_tag(&msg);

            // ApplicationMessage has a context field too, so in fact every
            // variant present today should expose one. Asserted uniformly
            // rather than by an allow-list, so the question a new variant
            // raises is "does it have a context", not "is it in the list".
            let _ = carries_context;

            assert!(
                msg.get_context().is_some(),
                "InboundDelegateMsg tag {tag} has a context field but get_context returned None; \
                 the `_ => None` wildcard hides a missing arm"
            );
            assert!(
                msg.get_mut_context().is_some(),
                "InboundDelegateMsg tag {tag} has a context field but get_mut_context returned \
                 None; the two accessors must agree"
            );
        }
    }

    /// The same, for the outbound side.
    ///
    /// `RequestUserInput` and `ContextUpdated` genuinely have no context field
    /// to return, so they are the two exceptions and are named explicitly
    /// rather than skipped by a wildcard.
    #[test]
    fn every_outbound_variant_with_a_context_exposes_it() {
        for mut msg in every_outbound() {
            let tag = pinned_outbound_tag(&msg);
            let has_no_context = matches!(
                msg,
                OutboundDelegateMsg::RequestUserInput(_) | OutboundDelegateMsg::ContextUpdated(_)
            );
            if has_no_context {
                continue;
            }
            assert!(
                msg.get_context().is_some(),
                "OutboundDelegateMsg tag {tag} has a context field but get_context returned None"
            );
            assert!(
                msg.get_mut_context().is_some(),
                "OutboundDelegateMsg tag {tag} has a context field but get_mut_context returned \
                 None; the two accessors must agree"
            );
        }
    }
}
