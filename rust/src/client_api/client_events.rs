use bytes::Bytes;
use flatbuffers::WIPOffset;
use std::borrow::Cow;
use std::fmt::Display;
use std::net::SocketAddr;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::client_api::TryFromFbs;
use crate::generated::client_request::{
    root_as_client_request, ClientRequestType, ContractRequest as FbsContractRequest,
    ContractRequestType, DelegateRequest as FbsDelegateRequest, DelegateRequestType,
};

use crate::generated::common::{
    ApplicationMessage as FbsApplicationMessage, ApplicationMessageArgs, ContractCode,
    ContractCodeArgs, ContractContainer as FbsContractContainer, ContractContainerArgs,
    ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs,
    ContractKey as FbsContractKey, ContractKeyArgs, ContractType, DeltaUpdate, DeltaUpdateArgs,
    RelatedDeltaUpdate, RelatedDeltaUpdateArgs, RelatedStateAndDeltaUpdate,
    RelatedStateAndDeltaUpdateArgs, RelatedStateUpdate, RelatedStateUpdateArgs,
    StateAndDeltaUpdate, StateAndDeltaUpdateArgs, StateUpdate, StateUpdateArgs,
    UpdateData as FbsUpdateData, UpdateDataArgs, UpdateDataType, WasmContractV1,
    WasmContractV1Args,
};
use crate::generated::host_response::{
    finish_host_response_buffer, ClientResponse as FbsClientResponse, ClientResponseArgs,
    ContextUpdated as FbsContextUpdated, ContextUpdatedArgs,
    ContractResponse as FbsContractResponse, ContractResponseArgs, ContractResponseType,
    DelegateKey as FbsDelegateKey, DelegateKeyArgs, DelegateResponse as FbsDelegateResponse,
    DelegateResponseArgs, GetResponse as FbsGetResponse, GetResponseArgs,
    HostResponse as FbsHostResponse, HostResponseArgs, HostResponseType, NotFound as FbsNotFound,
    NotFoundArgs, Ok as FbsOk, OkArgs, OutboundDelegateMsg as FbsOutboundDelegateMsg,
    OutboundDelegateMsgArgs, OutboundDelegateMsgType, PutResponse as FbsPutResponse,
    PutResponseArgs, RequestUserInput as FbsRequestUserInput, RequestUserInputArgs,
    StreamChunk as FbsHostStreamChunk, StreamChunkArgs as FbsHostStreamChunkArgs,
    UpdateNotification as FbsUpdateNotification, UpdateNotificationArgs,
    UpdateResponse as FbsUpdateResponse, UpdateResponseArgs,
};
use crate::prelude::ContractContainer::Wasm;
use crate::prelude::ContractWasmAPIVersion::V1;
use crate::prelude::UpdateData::{
    Delta, RelatedDelta, RelatedState, RelatedStateAndDelta, State, StateAndDelta,
};
use crate::{
    delegate_interface::{DelegateKey, InboundDelegateMsg, OutboundDelegateMsg},
    prelude::{
        ContractInstanceId, ContractKey, DelegateContainer, Parameters, RelatedContracts,
        SecretsId, StateSummary, UpdateData, WrappedState,
    },
    versioning::ContractContainer,
};

use super::WsApiError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientError {
    kind: Box<ErrorKind>,
}

impl ClientError {
    pub fn into_fbs_bytes(self) -> Result<Vec<u8>, Box<ClientError>> {
        use crate::generated::host_response::{Error, ErrorArgs};
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let msg_offset = builder.create_string(&self.to_string());
        let err_offset = Error::create(
            &mut builder,
            &ErrorArgs {
                msg: Some(msg_offset),
            },
        );
        let host_response_offset = FbsHostResponse::create(
            &mut builder,
            &HostResponseArgs {
                response_type: HostResponseType::Ok,
                response: Some(err_offset.as_union_value()),
            },
        );
        finish_host_response_buffer(&mut builder, host_response_offset);
        Ok(builder.finished_data().to_vec())
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl From<ErrorKind> for ClientError {
    fn from(kind: ErrorKind) -> Self {
        ClientError {
            kind: Box::new(kind),
        }
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for ClientError {
    fn from(cause: T) -> Self {
        ClientError {
            kind: Box::new(ErrorKind::Unhandled {
                cause: cause.into(),
            }),
        }
    }
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize, Clone)]
#[non_exhaustive]
pub enum ErrorKind {
    #[error("comm channel between client/host closed")]
    ChannelClosed,
    #[error("error while deserializing: {cause}")]
    DeserializationError { cause: Cow<'static, str> },
    #[error("client disconnected")]
    Disconnect,
    #[error("failed while trying to unpack state for {0}")]
    IncorrectState(ContractKey),
    #[error("node not available")]
    NodeUnavailable,
    #[error("lost the connection with the protocol handling connections")]
    TransportProtocolDisconnect,
    #[error("unhandled error: {cause}")]
    Unhandled { cause: Cow<'static, str> },
    #[error("unknown client id: {0}")]
    UnknownClient(usize),
    #[error(transparent)]
    RequestError(#[from] RequestError),
    #[error("error while executing operation in the network: {cause}")]
    OperationError { cause: Cow<'static, str> },
    // TODO: identify requests by some id so we can inform clients which one failed exactly
    #[error("operation timed out")]
    FailedOperation,
    #[error("peer should shutdown")]
    Shutdown,
    #[error("no ring connections found")]
    EmptyRing,
    #[error("peer has not joined the network yet")]
    PeerNotJoined,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "client error: {}", self.kind)
    }
}

impl std::error::Error for ClientError {}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone)]
#[non_exhaustive]
pub enum RequestError {
    #[error(transparent)]
    ContractError(#[from] ContractError),
    #[error(transparent)]
    DelegateError(#[from] DelegateError),
    #[error("client disconnect")]
    Disconnect,
    #[error("operation timed out")]
    Timeout,
}

/// Errors that may happen while interacting with delegates.
#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone)]
#[non_exhaustive]
pub enum DelegateError {
    #[error("error while registering delegate {0}")]
    RegisterError(DelegateKey),
    #[error("execution error, cause {0}")]
    ExecutionError(Cow<'static, str>),
    #[error("missing delegate {0}")]
    Missing(DelegateKey),
    #[error("missing secret `{secret}` for delegate {key}")]
    MissingSecret { key: DelegateKey, secret: SecretsId },
    #[error("forbidden access to secret: {0}")]
    ForbiddenSecretAccess(SecretsId),
}

/// Errors that may happen while interacting with contracts.
#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone)]
#[non_exhaustive]
pub enum ContractError {
    #[error("failed to get contract {key}, reason: {cause}")]
    Get {
        key: ContractKey,
        cause: Cow<'static, str>,
    },
    #[error("put error for contract {key}, reason: {cause}")]
    Put {
        key: ContractKey,
        cause: Cow<'static, str>,
    },
    #[error("update error for contract {key}, reason: {cause}")]
    Update {
        key: ContractKey,
        cause: Cow<'static, str>,
    },
    #[error("failed to subscribe for contract {key}, reason: {cause}")]
    Subscribe {
        key: ContractKey,
        cause: Cow<'static, str>,
    },
    // todo: actually build a stack of the involved keys
    #[error("dependency contract stack overflow : {key}")]
    ContractStackOverflow {
        key: crate::contract_interface::ContractInstanceId,
    },
    #[error("missing related contract: {key}")]
    MissingRelated {
        key: crate::contract_interface::ContractInstanceId,
    },
    #[error("missing contract: {key}")]
    MissingContract {
        key: crate::contract_interface::ContractInstanceId,
    },
}

impl ContractError {
    const EXECUTION_ERROR: &'static str = "execution error";
    const INVALID_PUT: &'static str = "invalid put";

    pub fn update_exec_error(key: ContractKey, additional_info: impl std::fmt::Display) -> Self {
        Self::Update {
            key,
            cause: format!(
                "{exec_err}: {additional_info}",
                exec_err = Self::EXECUTION_ERROR
            )
            .into(),
        }
    }

    pub fn invalid_put(key: ContractKey) -> Self {
        Self::Put {
            key,
            cause: Self::INVALID_PUT.into(),
        }
    }

    pub fn invalid_update(key: ContractKey) -> Self {
        Self::Update {
            key,
            cause: Self::INVALID_PUT.into(),
        }
    }
}

/// A request from a client application to the host.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
// #[cfg_attr(test, derive(arbitrary::Arbitrary))]
pub enum ClientRequest<'a> {
    DelegateOp(#[serde(borrow)] DelegateRequest<'a>),
    ContractOp(#[serde(borrow)] ContractRequest<'a>),
    Disconnect {
        cause: Option<Cow<'static, str>>,
    },
    Authenticate {
        token: String,
    },
    NodeQueries(NodeQuery),
    /// Gracefully disconnect from the host.
    Close,
    /// A chunk of a larger streamed message.
    StreamChunk {
        stream_id: u32,
        index: u32,
        total: u32,
        data: Bytes,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectedPeers {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeDiagnostics {
    /// Optional contract key to filter diagnostics for specific contract
    pub contract_key: Option<ContractKey>,
}

impl ClientRequest<'_> {
    pub fn into_owned(self) -> ClientRequest<'static> {
        match self {
            ClientRequest::ContractOp(op) => {
                let owned = match op {
                    ContractRequest::Put {
                        contract,
                        state,
                        related_contracts,
                        subscribe,
                        blocking_subscribe,
                    } => {
                        let related_contracts = related_contracts.into_owned();
                        ContractRequest::Put {
                            contract,
                            state,
                            related_contracts,
                            subscribe,
                            blocking_subscribe,
                        }
                    }
                    ContractRequest::Update { key, data } => {
                        let data = data.into_owned();
                        ContractRequest::Update { key, data }
                    }
                    ContractRequest::Get {
                        key,
                        return_contract_code,
                        subscribe,
                        blocking_subscribe,
                    } => ContractRequest::Get {
                        key,
                        return_contract_code,
                        subscribe,
                        blocking_subscribe,
                    },
                    ContractRequest::Subscribe { key, summary } => ContractRequest::Subscribe {
                        key,
                        summary: summary.map(StateSummary::into_owned),
                    },
                };
                owned.into()
            }
            ClientRequest::DelegateOp(op) => {
                let op = op.into_owned();
                ClientRequest::DelegateOp(op)
            }
            ClientRequest::Disconnect { cause } => ClientRequest::Disconnect { cause },
            ClientRequest::Authenticate { token } => ClientRequest::Authenticate { token },
            ClientRequest::NodeQueries(query) => ClientRequest::NodeQueries(query),
            ClientRequest::Close => ClientRequest::Close,
            ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data,
            } => ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data,
            },
        }
    }

    pub fn is_disconnect(&self) -> bool {
        matches!(self, Self::Disconnect { .. })
    }

    pub fn try_decode_fbs(msg: &[u8]) -> Result<ClientRequest<'_>, WsApiError> {
        let req = {
            match root_as_client_request(msg) {
                Ok(client_request) => match client_request.client_request_type() {
                    ClientRequestType::ContractRequest => {
                        let contract_request =
                            client_request.client_request_as_contract_request().unwrap();
                        ContractRequest::try_decode_fbs(&contract_request)?.into()
                    }
                    ClientRequestType::DelegateRequest => {
                        let delegate_request =
                            client_request.client_request_as_delegate_request().unwrap();
                        DelegateRequest::try_decode_fbs(&delegate_request)?.into()
                    }
                    ClientRequestType::Disconnect => {
                        let delegate_request =
                            client_request.client_request_as_disconnect().unwrap();
                        let cause = delegate_request
                            .cause()
                            .map(|cause_msg| cause_msg.to_string().into());
                        ClientRequest::Disconnect { cause }
                    }
                    ClientRequestType::Authenticate => {
                        let auth_req = client_request.client_request_as_authenticate().unwrap();
                        let token = auth_req.token();
                        ClientRequest::Authenticate {
                            token: token.to_owned(),
                        }
                    }
                    ClientRequestType::StreamChunk => {
                        let chunk = client_request.client_request_as_stream_chunk().unwrap();
                        ClientRequest::StreamChunk {
                            stream_id: chunk.stream_id(),
                            index: chunk.index(),
                            total: chunk.total(),
                            data: Bytes::from(chunk.data().bytes().to_vec()),
                        }
                    }
                    other => {
                        return Err(crate::client_api::unknown_union_discriminant(
                            "ClientRequestType",
                            other.0,
                        ))
                    }
                },
                Err(e) => {
                    let cause = format!("{e}");
                    return Err(WsApiError::deserialization(cause));
                }
            }
        };

        Ok(req)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContractRequest<'a> {
    /// Insert a new value in a contract corresponding with the provided key.
    Put {
        contract: ContractContainer,
        /// Value to upsert in the contract.
        state: WrappedState,
        /// Related contracts.
        #[serde(borrow)]
        related_contracts: RelatedContracts<'a>,
        /// If this flag is set then subscribe to updates for this contract.
        subscribe: bool,
        /// If true, the PUT response waits for the subscription to complete.
        /// Only meaningful when `subscribe` is true.
        #[serde(default)]
        blocking_subscribe: bool,
    },
    /// Update an existing contract corresponding with the provided key.
    Update {
        key: ContractKey,
        #[serde(borrow)]
        data: UpdateData<'a>,
    },
    /// Fetch the current state from a contract corresponding to the provided key.
    Get {
        /// Instance ID of the contract (the hash of code + params).
        /// Only the instance ID is needed since the client doesn't have the code hash yet.
        key: ContractInstanceId,
        /// If this flag is set then fetch also the contract itself.
        return_contract_code: bool,
        /// If this flag is set then subscribe to updates for this contract.
        subscribe: bool,
        /// If true, the GET response waits for the subscription to complete.
        /// Only meaningful when `subscribe` is true.
        #[serde(default)]
        blocking_subscribe: bool,
    },
    /// Subscribe to the changes in a given contract. Implicitly starts a get operation
    /// if the contract is not present yet.
    Subscribe {
        /// Instance ID of the contract.
        key: ContractInstanceId,
        summary: Option<StateSummary<'a>>,
    },
}

impl ContractRequest<'_> {
    pub fn into_owned(self) -> ContractRequest<'static> {
        match self {
            Self::Put {
                contract,
                state,
                related_contracts,
                subscribe,
                blocking_subscribe,
            } => ContractRequest::Put {
                contract,
                state,
                related_contracts: related_contracts.into_owned(),
                subscribe,
                blocking_subscribe,
            },
            Self::Update { key, data } => ContractRequest::Update {
                key,
                data: data.into_owned(),
            },
            Self::Get {
                key,
                return_contract_code: fetch_contract,
                subscribe,
                blocking_subscribe,
            } => ContractRequest::Get {
                key,
                return_contract_code: fetch_contract,
                subscribe,
                blocking_subscribe,
            },
            Self::Subscribe { key, summary } => ContractRequest::Subscribe {
                key,
                summary: summary.map(StateSummary::into_owned),
            },
        }
    }
}

impl<'a> From<ContractRequest<'a>> for ClientRequest<'a> {
    fn from(op: ContractRequest<'a>) -> Self {
        ClientRequest::ContractOp(op)
    }
}

/// Deserializes a `ContractRequest` from a Flatbuffers message.
impl<'a> TryFromFbs<&FbsContractRequest<'a>> for ContractRequest<'a> {
    fn try_decode_fbs(request: &FbsContractRequest<'a>) -> Result<Self, WsApiError> {
        let req = {
            match request.contract_request_type() {
                ContractRequestType::Get => {
                    let get = request.contract_request_as_get().unwrap();
                    // Extract just the instance ID - GET only needs the instance ID,
                    // not the full key (which may not be complete on the client side)
                    let fbs_key = get.key();
                    let key = crate::contract_interface::key::instance_id_from_fbs(
                        "ContractKey.instance.data",
                        fbs_key.instance().data().bytes(),
                    )?;
                    let fetch_contract = get.fetch_contract();
                    let subscribe = get.subscribe();
                    let blocking_subscribe = get.blocking_subscribe();
                    ContractRequest::Get {
                        key,
                        return_contract_code: fetch_contract,
                        subscribe,
                        blocking_subscribe,
                    }
                }
                ContractRequestType::Put => {
                    let put = request.contract_request_as_put().unwrap();
                    let contract = ContractContainer::try_decode_fbs(&put.container())?;
                    let state = WrappedState::new(put.wrapped_state().bytes().to_vec());
                    let related_contracts =
                        RelatedContracts::try_decode_fbs(&put.related_contracts())?.into_owned();
                    let subscribe = put.subscribe();
                    let blocking_subscribe = put.blocking_subscribe();
                    ContractRequest::Put {
                        contract,
                        state,
                        related_contracts,
                        subscribe,
                        blocking_subscribe,
                    }
                }
                ContractRequestType::Update => {
                    let update = request.contract_request_as_update().unwrap();
                    let key = ContractKey::try_decode_fbs(&update.key())?;
                    let data = UpdateData::try_decode_fbs(&update.data())?.into_owned();
                    ContractRequest::Update { key, data }
                }
                ContractRequestType::Subscribe => {
                    let subscribe = request.contract_request_as_subscribe().unwrap();
                    // Extract just the instance ID for Subscribe
                    let fbs_key = subscribe.key();
                    let key = crate::contract_interface::key::instance_id_from_fbs(
                        "ContractKey.instance.data",
                        fbs_key.instance().data().bytes(),
                    )?;
                    let summary = subscribe
                        .summary()
                        .map(|summary_data| StateSummary::from(summary_data.bytes()));
                    ContractRequest::Subscribe { key, summary }
                }
                // Reachable, not `unreachable!()`: the generated flatbuffers
                // verifier accepts any unknown union discriminant (`_ => Ok(())`)
                // and the union type field is a raw `u8` a client can set freely,
                // so a crafted request reaches here. Return a per-request error
                // instead of panicking the connection handler. (Mirrors the same
                // fix on `DelegateRequest::try_decode_fbs`.)
                other => {
                    return Err(crate::client_api::unknown_union_discriminant(
                        "ContractRequestType",
                        other.0,
                    ));
                }
            }
        };

        Ok(req)
    }
}

impl<'a> From<DelegateRequest<'a>> for ClientRequest<'a> {
    fn from(op: DelegateRequest<'a>) -> Self {
        ClientRequest::DelegateOp(op)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum DelegateRequest<'a> {
    ApplicationMessages {
        key: DelegateKey,
        #[serde(deserialize_with = "Parameters::deser_params")]
        params: Parameters<'a>,
        #[serde(borrow)]
        inbound: Vec<InboundDelegateMsg<'a>>,
    },
    RegisterDelegate {
        delegate: DelegateContainer,
        cipher: [u8; 32],
        nonce: [u8; 24],
    },
    UnregisterDelegate(DelegateKey),
    // Do NOT re-add a `RegisterDelegateWithPredecessors`-shaped variant
    // (predecessor keys + node-side secret copy-forward) without first
    // designing a non-forgeable way to attest which web-app is driving the
    // registration. The 0.8.4 version of this variant trusted the caller's
    // `origin_contract`, which turned out to be mintable by any HTTP client
    // for an arbitrary contract id — see freenet-core#5198 for the exploit
    // chain and freenet-core#5199 for the fix (disabling the node-side
    // handler) that predates this variant's removal here in 0.9.0.
}

impl DelegateRequest<'_> {
    pub fn into_owned(self) -> DelegateRequest<'static> {
        match self {
            DelegateRequest::ApplicationMessages {
                key,
                inbound,
                params,
            } => DelegateRequest::ApplicationMessages {
                key,
                params: params.into_owned(),
                inbound: inbound.into_iter().map(|e| e.into_owned()).collect(),
            },
            DelegateRequest::RegisterDelegate {
                delegate,
                cipher,
                nonce,
            } => DelegateRequest::RegisterDelegate {
                delegate,
                cipher,
                nonce,
            },
            DelegateRequest::UnregisterDelegate(key) => DelegateRequest::UnregisterDelegate(key),
        }
    }

    pub fn key(&self) -> &DelegateKey {
        match self {
            DelegateRequest::ApplicationMessages { key, .. } => key,
            DelegateRequest::RegisterDelegate { delegate, .. } => delegate.key(),
            DelegateRequest::UnregisterDelegate(key) => key,
        }
    }
}

impl Display for ClientRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientRequest::ContractOp(op) => match op {
                ContractRequest::Put {
                    contract, state, ..
                } => {
                    write!(
                        f,
                        "ContractRequest::Put for contract `{contract}` with state {state}"
                    )
                }
                ContractRequest::Update { key, .. } => write!(f, "update request for {key}"),
                ContractRequest::Get {
                    key,
                    return_contract_code: contract,
                    ..
                } => {
                    write!(
                        f,
                        "ContractRequest::Get for key `{key}` (fetch full contract: {contract})"
                    )
                }
                ContractRequest::Subscribe { key, .. } => {
                    write!(f, "ContractRequest::Subscribe for `{key}`")
                }
            },
            ClientRequest::DelegateOp(op) => match op {
                DelegateRequest::ApplicationMessages { key, inbound, .. } => {
                    write!(
                        f,
                        "DelegateRequest::ApplicationMessages for `{key}` with {} messages",
                        inbound.len()
                    )
                }
                DelegateRequest::RegisterDelegate { delegate, .. } => {
                    write!(
                        f,
                        "DelegateRequest::RegisterDelegate for delegate.key()=`{}`",
                        delegate.key()
                    )
                }
                DelegateRequest::UnregisterDelegate(key) => {
                    write!(f, "DelegateRequest::UnregisterDelegate for key `{key}`")
                }
            },
            ClientRequest::Disconnect { .. } => write!(f, "client disconnected"),
            ClientRequest::Authenticate { .. } => write!(f, "authenticate"),
            ClientRequest::NodeQueries(query) => write!(f, "node queries: {:?}", query),
            ClientRequest::Close => write!(f, "close"),
            ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                ..
            } => write!(f, "stream chunk {index}/{total} (stream {stream_id})"),
        }
    }
}

/// Deserializes a `DelegateRequest` from a Flatbuffers message.
impl<'a> TryFromFbs<&FbsDelegateRequest<'a>> for DelegateRequest<'a> {
    fn try_decode_fbs(request: &FbsDelegateRequest<'a>) -> Result<Self, WsApiError> {
        let req = {
            match request.delegate_request_type() {
                DelegateRequestType::ApplicationMessages => {
                    let app_msg = request.delegate_request_as_application_messages().unwrap();
                    let key = DelegateKey::try_decode_fbs(&app_msg.key())?;
                    let params = Parameters::from(app_msg.params().bytes());
                    let inbound = app_msg
                        .inbound()
                        .iter()
                        .map(|msg| InboundDelegateMsg::try_decode_fbs(&msg))
                        .collect::<Result<Vec<_>, _>>()?;
                    DelegateRequest::ApplicationMessages {
                        key,
                        params,
                        inbound,
                    }
                }
                DelegateRequestType::RegisterDelegate => {
                    let register = request.delegate_request_as_register_delegate().unwrap();
                    let delegate = DelegateContainer::try_decode_fbs(&register.delegate())?;
                    // `cipher` and `nonce` are `(required)`, which the verifier
                    // reads as "present", not "32 and 24 bytes". The
                    // `try_from(..).unwrap()` this replaces panicked on any
                    // other length, killing the connection task. (The
                    // intermediate `.to_vec()` went with it: `bytes()` is
                    // already a slice.)
                    let cipher = crate::client_api::fixed_size_field::<32>(
                        "RegisterDelegate.cipher",
                        register.cipher().bytes(),
                    )?;
                    let nonce = crate::client_api::fixed_size_field::<24>(
                        "RegisterDelegate.nonce",
                        register.nonce().bytes(),
                    )?;
                    DelegateRequest::RegisterDelegate {
                        delegate,
                        cipher,
                        nonce,
                    }
                }
                DelegateRequestType::UnregisterDelegate => {
                    let unregister = request.delegate_request_as_unregister_delegate().unwrap();
                    let key = DelegateKey::try_decode_fbs(&unregister.key())?;
                    DelegateRequest::UnregisterDelegate(key)
                }
                // An unknown union discriminant is reachable, not `unreachable!()`:
                // the generated flatbuffers verifier accepts any discriminant it
                // doesn't recognize (`_ => Ok(())`), and the union type field is a
                // raw `u8` the (public) TS builder can set to any value. Panicking
                // here would let a single crafted request take down the connection
                // handler, so return a per-request error instead.
                other => {
                    return Err(crate::client_api::unknown_union_discriminant(
                        "DelegateRequestType",
                        other.0,
                    ));
                }
            }
        };

        Ok(req)
    }
}

/// A response to a previous [`ClientRequest`]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum HostResponse<T = WrappedState> {
    ContractResponse(#[serde(bound(deserialize = "T: DeserializeOwned"))] ContractResponse<T>),
    DelegateResponse {
        key: DelegateKey,
        values: Vec<OutboundDelegateMsg>,
    },
    QueryResponse(QueryResponse),
    /// A requested action which doesn't require an answer was performed successfully.
    Ok,
    /// A chunk of a larger streamed response.
    StreamChunk {
        stream_id: u32,
        index: u32,
        total: u32,
        data: Bytes,
    },
    /// Header message announcing the start of a streamed response.
    /// Sent before the corresponding [`StreamChunk`] messages so the client
    /// can set up incremental consumption via [`WsStreamHandle`].
    StreamHeader {
        stream_id: u32,
        total_bytes: u64,
        content: StreamContent,
    },
}

/// Describes what kind of response is being streamed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StreamContent {
    /// A streamed GetResponse — the large state is delivered via StreamChunks.
    GetResponse {
        key: ContractKey,
        includes_contract: bool,
    },
    /// Raw binary stream (future use).
    Raw,
}

type Peer = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueryResponse {
    ConnectedPeers { peers: Vec<(Peer, SocketAddr)> },
    NetworkDebug(NetworkDebugInfo),
    NodeDiagnostics(NodeDiagnosticsResponse),
    NeighborHosting(NeighborHostingInfo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkDebugInfo {
    pub subscriptions: Vec<SubscriptionInfo>,
    pub connected_peers: Vec<(String, SocketAddr)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeDiagnosticsResponse {
    /// Node information
    pub node_info: Option<NodeInfo>,

    /// Network connectivity information
    pub network_info: Option<NetworkInfo>,

    /// Contract subscription information
    pub subscriptions: Vec<SubscriptionInfo>,

    /// Contract states for specific contracts.
    ///
    /// Keys are the Base58-encoded contract id (i.e. `ContractKey::Display`),
    /// matching the convention every other field in this struct uses
    /// (`peer_id: String`, `connected_peers: Vec<(String, String)>`,
    /// `ContractHostingEntry::contract_key: String`). Pre-0.7 this was
    /// `HashMap<ContractKey, ContractState>`, which `serde_json` could not
    /// serialize because JSON object keys must be strings — every report
    /// from a node hosting at least one contract had its
    /// `network_status` silently dropped. See freenet/freenet-core#3987.
    pub contract_states: std::collections::HashMap<String, ContractState>,

    /// System metrics
    pub system_metrics: Option<SystemMetrics>,

    /// Information about connected peers with detailed data
    pub connected_peers_detailed: Vec<ConnectedPeerInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub peer_id: String,
    pub is_gateway: bool,
    pub location: Option<String>,
    pub listening_address: Option<String>,
    pub uptime_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInfo {
    pub connected_peers: Vec<(String, String)>, // (peer_id, address)
    pub active_connections: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractState {
    /// Number of nodes subscribed to this contract
    pub subscribers: u32,
    /// Peer IDs of nodes that are subscribed to this contract
    pub subscriber_peer_ids: Vec<String>,
    /// Size of the contract state in bytes
    #[serde(default)]
    pub size_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemMetrics {
    pub active_connections: u32,
    pub hosting_contracts: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscriptionInfo {
    pub contract_key: ContractInstanceId,
    pub client_id: usize,
}

/// Basic information about a connected peer
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectedPeerInfo {
    pub peer_id: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeQuery {
    ConnectedPeers,
    SubscriptionInfo,
    NodeDiagnostics {
        /// Diagnostic configuration specifying what information to collect
        config: NodeDiagnosticsConfig,
    },
    /// Query neighbor hosting information for update propagation
    NeighborHostingInfo,
}

/// Neighbor hosting information for update propagation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NeighborHostingInfo {
    /// Contracts this node is currently hosting
    pub my_hosted: Vec<ContractHostingEntry>,
    /// What we know about neighbor hosting
    pub neighbor_hosting: Vec<NeighborHostingDetail>,
    /// Hosting propagation statistics
    pub stats: HostingStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractHostingEntry {
    /// Full contract key as string
    pub contract_key: String,
    /// 32-bit hash for proximity matching
    pub hosting_hash: u32,
    /// When this contract was first hosted (Unix timestamp)
    pub hosted_since: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NeighborHostingDetail {
    /// Peer identifier
    pub peer_id: String,
    /// Contract hashes this neighbor is known to host
    pub known_contracts: Vec<u32>,
    /// Last update received from this neighbor (Unix timestamp)
    pub last_update: u64,
    /// Number of updates received from this neighbor
    pub update_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostingStats {
    /// Number of hosting announcements sent
    pub hosting_announces_sent: u64,
    /// Number of hosting announcements received
    pub hosting_announces_received: u64,
    /// Updates forwarded via proximity (not subscription)
    pub updates_via_proximity: u64,
    /// Updates forwarded via subscription
    pub updates_via_subscription: u64,
    /// False positives due to hash collisions
    pub false_positive_forwards: u64,
    /// Average number of contracts per neighbor
    pub avg_neighbor_hosting_size: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeDiagnosticsConfig {
    /// Include basic node information (ID, location, uptime, etc.)
    pub include_node_info: bool,

    /// Include network connectivity information
    pub include_network_info: bool,

    /// Include contract subscription information
    pub include_subscriptions: bool,

    /// Include contract states for specific contracts (empty = all contracts)
    pub contract_keys: Vec<ContractKey>,

    /// Include memory and performance metrics
    pub include_system_metrics: bool,

    /// Include detailed information about connected peers (vs basic peer list)
    pub include_detailed_peer_info: bool,

    /// Include peer IDs of subscribers in contract state information
    pub include_subscriber_peer_ids: bool,
}

impl NodeDiagnosticsConfig {
    /// Create a comprehensive diagnostic config for debugging update propagation issues
    pub fn for_update_propagation_debugging(contract_key: ContractKey) -> Self {
        Self {
            include_node_info: true,
            include_network_info: true,
            include_subscriptions: true,
            contract_keys: vec![contract_key],
            include_system_metrics: true,
            include_detailed_peer_info: true,
            include_subscriber_peer_ids: true,
        }
    }

    /// Create a lightweight diagnostic config for basic node status
    pub fn basic_status() -> Self {
        Self {
            include_node_info: true,
            include_network_info: true,
            include_subscriptions: false,
            contract_keys: vec![],
            include_system_metrics: false,
            include_detailed_peer_info: false,
            include_subscriber_peer_ids: false,
        }
    }

    /// Create a full diagnostic config (all information)
    pub fn full() -> Self {
        Self {
            include_node_info: true,
            include_network_info: true,
            include_subscriptions: true,
            contract_keys: vec![], // empty = all contracts
            include_system_metrics: true,
            include_detailed_peer_info: true,
            include_subscriber_peer_ids: true,
        }
    }
}

impl HostResponse {
    pub fn unwrap_put(self) -> ContractKey {
        if let Self::ContractResponse(ContractResponse::PutResponse { key }) = self {
            key
        } else {
            panic!("called `HostResponse::unwrap_put()` on other than `PutResponse` value")
        }
    }

    pub fn unwrap_get(self) -> (WrappedState, Option<ContractContainer>) {
        if let Self::ContractResponse(ContractResponse::GetResponse {
            contract, state, ..
        }) = self
        {
            (state, contract)
        } else {
            panic!("called `HostResponse::unwrap_put()` on other than `PutResponse` value")
        }
    }

    pub fn into_fbs_bytes(self) -> Result<Vec<u8>, Box<ClientError>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        match self {
            HostResponse::ContractResponse(res) => match res {
                ContractResponse::PutResponse { key } => {
                    let instance_data = builder.create_vector(key.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );

                    let code = Some(builder.create_vector(&key.code_hash().0));
                    let key_offset = FbsContractKey::create(
                        &mut builder,
                        &ContractKeyArgs {
                            instance: Some(instance_offset),
                            code,
                        },
                    );

                    let put_offset = FbsPutResponse::create(
                        &mut builder,
                        &PutResponseArgs {
                            key: Some(key_offset),
                        },
                    );

                    let contract_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response: Some(put_offset.as_union_value()),
                            contract_response_type: ContractResponseType::PutResponse,
                        },
                    );

                    let response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response: Some(contract_response_offset.as_union_value()),
                            response_type: HostResponseType::ContractResponse,
                        },
                    );

                    finish_host_response_buffer(&mut builder, response_offset);
                    Ok(builder.finished_data().to_vec())
                }
                ContractResponse::UpdateResponse { key, summary } => {
                    let instance_data = builder.create_vector(key.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );

                    let code = Some(builder.create_vector(&key.code_hash().0));

                    let key_offset = FbsContractKey::create(
                        &mut builder,
                        &ContractKeyArgs {
                            instance: Some(instance_offset),
                            code,
                        },
                    );

                    let summary_data = builder.create_vector(&summary.into_bytes());

                    let update_response_offset = FbsUpdateResponse::create(
                        &mut builder,
                        &UpdateResponseArgs {
                            key: Some(key_offset),
                            summary: Some(summary_data),
                        },
                    );

                    let contract_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response: Some(update_response_offset.as_union_value()),
                            contract_response_type: ContractResponseType::UpdateResponse,
                        },
                    );

                    let response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response: Some(contract_response_offset.as_union_value()),
                            response_type: HostResponseType::ContractResponse,
                        },
                    );

                    finish_host_response_buffer(&mut builder, response_offset);
                    Ok(builder.finished_data().to_vec())
                }
                ContractResponse::GetResponse {
                    key,
                    contract: contract_container,
                    state,
                } => {
                    let instance_data = builder.create_vector(key.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );

                    let code = Some(builder.create_vector(&key.code_hash().0));
                    let key_offset = FbsContractKey::create(
                        &mut builder,
                        &ContractKeyArgs {
                            instance: Some(instance_offset),
                            code,
                        },
                    );

                    let container_offset = if let Some(contract) = contract_container {
                        let data = builder.create_vector(contract.key().as_bytes());

                        let instance_offset = FbsContractInstanceId::create(
                            &mut builder,
                            &ContractInstanceIdArgs { data: Some(data) },
                        );

                        let code = Some(builder.create_vector(&contract.key().code_hash().0));
                        let contract_key_offset = FbsContractKey::create(
                            &mut builder,
                            &ContractKeyArgs {
                                instance: Some(instance_offset),
                                code,
                            },
                        );

                        let contract_data =
                            builder.create_vector(contract.clone().unwrap_v1().data.data());
                        let contract_code_hash =
                            builder.create_vector(&contract.clone().unwrap_v1().data.hash().0);

                        let contract_code_offset = ContractCode::create(
                            &mut builder,
                            &ContractCodeArgs {
                                data: Some(contract_data),
                                code_hash: Some(contract_code_hash),
                            },
                        );

                        let contract_params =
                            builder.create_vector(&contract.clone().params().into_bytes());

                        let contract_offset = match contract {
                            Wasm(V1(..)) => WasmContractV1::create(
                                &mut builder,
                                &WasmContractV1Args {
                                    key: Some(contract_key_offset),
                                    data: Some(contract_code_offset),
                                    parameters: Some(contract_params),
                                },
                            ),
                        };

                        Some(FbsContractContainer::create(
                            &mut builder,
                            &ContractContainerArgs {
                                contract_type: ContractType::WasmContractV1,
                                contract: Some(contract_offset.as_union_value()),
                            },
                        ))
                    } else {
                        None
                    };

                    let state_data = builder.create_vector(&state);

                    let get_offset = FbsGetResponse::create(
                        &mut builder,
                        &GetResponseArgs {
                            key: Some(key_offset),
                            contract: container_offset,
                            state: Some(state_data),
                        },
                    );

                    let contract_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::GetResponse,
                            contract_response: Some(get_offset.as_union_value()),
                        },
                    );

                    let response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response: Some(contract_response_offset.as_union_value()),
                            response_type: HostResponseType::ContractResponse,
                        },
                    );

                    finish_host_response_buffer(&mut builder, response_offset);
                    Ok(builder.finished_data().to_vec())
                }
                ContractResponse::UpdateNotification { key, update } => {
                    let instance_data = builder.create_vector(key.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );

                    let code = Some(builder.create_vector(&key.code_hash().0));
                    let key_offset = FbsContractKey::create(
                        &mut builder,
                        &ContractKeyArgs {
                            instance: Some(instance_offset),
                            code,
                        },
                    );

                    let update_data = match update {
                        State(state) => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let state_update_offset = StateUpdate::create(
                                &mut builder,
                                &StateUpdateArgs {
                                    state: Some(state_data),
                                },
                            );
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateUpdate,
                                    update_data: Some(state_update_offset.as_union_value()),
                                },
                            )
                        }
                        Delta(delta) => {
                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let update_offset = DeltaUpdate::create(
                                &mut builder,
                                &DeltaUpdateArgs {
                                    delta: Some(delta_data),
                                },
                            );
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::DeltaUpdate,
                                    update_data: Some(update_offset.as_union_value()),
                                },
                            )
                        }
                        StateAndDelta { state, delta } => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let delta_data = builder.create_vector(&delta.into_bytes());

                            let update_offset = StateAndDeltaUpdate::create(
                                &mut builder,
                                &StateAndDeltaUpdateArgs {
                                    state: Some(state_data),
                                    delta: Some(delta_data),
                                },
                            );

                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateAndDeltaUpdate,
                                    update_data: Some(update_offset.as_union_value()),
                                },
                            )
                        }
                        RelatedState { related_to, state } => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            // RAW 32 bytes, like every other `common.ContractInstanceId`
                            // producer. This wrote `related_to.encode()` — base58
                            // TEXT — into a field the schema and the TypeScript
                            // SDK both read as raw bytes. It is the encode half of
                            // the decode bug this change fixes: the same field, the
                            // same wrong transformation, mirrored.
                            let instance_data = builder.create_vector(related_to.as_bytes());

                            let instance_offset = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs {
                                    data: Some(instance_data),
                                },
                            );

                            let update_offset = RelatedStateUpdate::create(
                                &mut builder,
                                &RelatedStateUpdateArgs {
                                    related_to: Some(instance_offset),
                                    state: Some(state_data),
                                },
                            );

                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::RelatedStateUpdate,
                                    update_data: Some(update_offset.as_union_value()),
                                },
                            )
                        }
                        RelatedDelta { related_to, delta } => {
                            // RAW 32 bytes, like every other `common.ContractInstanceId`
                            // producer. This wrote `related_to.encode()` — base58
                            // TEXT — into a field the schema and the TypeScript
                            // SDK both read as raw bytes. It is the encode half of
                            // the decode bug this change fixes: the same field, the
                            // same wrong transformation, mirrored.
                            let instance_data = builder.create_vector(related_to.as_bytes());
                            let delta_data = builder.create_vector(&delta.into_bytes());

                            let instance_offset = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs {
                                    data: Some(instance_data),
                                },
                            );

                            let update_offset = RelatedDeltaUpdate::create(
                                &mut builder,
                                &RelatedDeltaUpdateArgs {
                                    related_to: Some(instance_offset),
                                    delta: Some(delta_data),
                                },
                            );

                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::RelatedDeltaUpdate,
                                    update_data: Some(update_offset.as_union_value()),
                                },
                            )
                        }
                        RelatedStateAndDelta {
                            related_to,
                            state,
                            delta,
                        } => {
                            // RAW 32 bytes, like every other `common.ContractInstanceId`
                            // producer. This wrote `related_to.encode()` — base58
                            // TEXT — into a field the schema and the TypeScript
                            // SDK both read as raw bytes. It is the encode half of
                            // the decode bug this change fixes: the same field, the
                            // same wrong transformation, mirrored.
                            let instance_data = builder.create_vector(related_to.as_bytes());
                            let state_data = builder.create_vector(&state.into_bytes());
                            let delta_data = builder.create_vector(&delta.into_bytes());

                            let instance_offset = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs {
                                    data: Some(instance_data),
                                },
                            );

                            let update_offset = RelatedStateAndDeltaUpdate::create(
                                &mut builder,
                                &RelatedStateAndDeltaUpdateArgs {
                                    related_to: Some(instance_offset),
                                    state: Some(state_data),
                                    delta: Some(delta_data),
                                },
                            );

                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::RelatedStateAndDeltaUpdate,
                                    update_data: Some(update_offset.as_union_value()),
                                },
                            )
                        }
                    };

                    let update_notification_offset = FbsUpdateNotification::create(
                        &mut builder,
                        &UpdateNotificationArgs {
                            key: Some(key_offset),
                            update: Some(update_data),
                        },
                    );

                    let put_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::UpdateNotification,
                            contract_response: Some(update_notification_offset.as_union_value()),
                        },
                    );

                    let host_response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(put_response_offset.as_union_value()),
                        },
                    );

                    finish_host_response_buffer(&mut builder, host_response_offset);
                    Ok(builder.finished_data().to_vec())
                }
                ContractResponse::SubscribeResponse { key, .. } => {
                    // SubscribeResponse FBS type not yet in generated code,
                    // serialize as PutResponse (same shape: just a key) so
                    // the client receives a valid response instead of a crash.
                    let instance_data = builder.create_vector(key.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );
                    let code = Some(builder.create_vector(&key.code_hash().0));
                    let key_offset = FbsContractKey::create(
                        &mut builder,
                        &ContractKeyArgs {
                            instance: Some(instance_offset),
                            code,
                        },
                    );
                    let put_offset = FbsPutResponse::create(
                        &mut builder,
                        &PutResponseArgs {
                            key: Some(key_offset),
                        },
                    );
                    let contract_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::PutResponse,
                            contract_response: Some(put_offset.as_union_value()),
                        },
                    );
                    let host_response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(contract_response_offset.as_union_value()),
                        },
                    );
                    finish_host_response_buffer(&mut builder, host_response_offset);
                    Ok(builder.finished_data().to_vec())
                }
                ContractResponse::NotFound { instance_id } => {
                    let instance_data = builder.create_vector(instance_id.as_bytes());
                    let instance_offset = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs {
                            data: Some(instance_data),
                        },
                    );

                    let not_found_offset = FbsNotFound::create(
                        &mut builder,
                        &NotFoundArgs {
                            instance_id: Some(instance_offset),
                        },
                    );

                    let contract_response_offset = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::NotFound,
                            contract_response: Some(not_found_offset.as_union_value()),
                        },
                    );

                    let response_offset = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response: Some(contract_response_offset.as_union_value()),
                            response_type: HostResponseType::ContractResponse,
                        },
                    );

                    finish_host_response_buffer(&mut builder, response_offset);
                    Ok(builder.finished_data().to_vec())
                }
            },
            HostResponse::DelegateResponse { key, values } => {
                let key_data = builder.create_vector(key.bytes());
                let code_hash_data = builder.create_vector(&key.code_hash().0);
                let key_offset = FbsDelegateKey::create(
                    &mut builder,
                    &DelegateKeyArgs {
                        key: Some(key_data),
                        code_hash: Some(code_hash_data),
                    },
                );
                let mut messages: Vec<WIPOffset<FbsOutboundDelegateMsg>> = Vec::new();
                values.iter().for_each(|msg| match msg {
                    OutboundDelegateMsg::ApplicationMessage(app) => {
                        let payload_data = builder.create_vector(&app.payload);
                        let delegate_context_data = builder.create_vector(app.context.as_ref());
                        let app_offset = FbsApplicationMessage::create(
                            &mut builder,
                            &ApplicationMessageArgs {
                                payload: Some(payload_data),
                                context: Some(delegate_context_data),
                                processed: app.processed,
                            },
                        );
                        let msg = FbsOutboundDelegateMsg::create(
                            &mut builder,
                            &OutboundDelegateMsgArgs {
                                inbound_type: OutboundDelegateMsgType::common_ApplicationMessage,
                                inbound: Some(app_offset.as_union_value()),
                            },
                        );
                        messages.push(msg);
                    }
                    OutboundDelegateMsg::RequestUserInput(input) => {
                        let message_data = builder.create_vector(input.message.bytes());
                        let mut responses: Vec<WIPOffset<FbsClientResponse>> = Vec::new();
                        input.responses.iter().for_each(|resp| {
                            let response_data = builder.create_vector(resp.bytes());
                            let response = FbsClientResponse::create(
                                &mut builder,
                                &ClientResponseArgs {
                                    data: Some(response_data),
                                },
                            );
                            responses.push(response)
                        });
                        let responses_offset = builder.create_vector(&responses);
                        let input_offset = FbsRequestUserInput::create(
                            &mut builder,
                            &RequestUserInputArgs {
                                request_id: input.request_id,
                                message: Some(message_data),
                                responses: Some(responses_offset),
                            },
                        );
                        let msg = FbsOutboundDelegateMsg::create(
                            &mut builder,
                            &OutboundDelegateMsgArgs {
                                inbound_type: OutboundDelegateMsgType::RequestUserInput,
                                inbound: Some(input_offset.as_union_value()),
                            },
                        );
                        messages.push(msg);
                    }
                    OutboundDelegateMsg::ContextUpdated(context) => {
                        let context_data = builder.create_vector(context.as_ref());
                        let context_offset = FbsContextUpdated::create(
                            &mut builder,
                            &ContextUpdatedArgs {
                                context: Some(context_data),
                            },
                        );
                        let msg = FbsOutboundDelegateMsg::create(
                            &mut builder,
                            &OutboundDelegateMsgArgs {
                                inbound_type: OutboundDelegateMsgType::ContextUpdated,
                                inbound: Some(context_offset.as_union_value()),
                            },
                        );
                        messages.push(msg);
                    }
                    OutboundDelegateMsg::GetContractRequest(_) => {
                        // GetContractRequest should be handled by the executor and never
                        // reach client serialization. If we get here, it's a bug.
                        tracing::error!(
                            "GetContractRequest reached client serialization - this is a bug"
                        );
                    }
                    OutboundDelegateMsg::PutContractRequest(_) => {
                        // PutContractRequest should be handled by the executor and never
                        // reach client serialization. If we get here, it's a bug.
                        tracing::error!(
                            "PutContractRequest reached client serialization - this is a bug"
                        );
                    }
                    OutboundDelegateMsg::UpdateContractRequest(_) => {
                        tracing::error!(
                            "UpdateContractRequest reached client serialization - this is a bug"
                        );
                    }
                    OutboundDelegateMsg::SubscribeContractRequest(_) => {
                        tracing::error!(
                            "SubscribeContractRequest reached client serialization - this is a bug"
                        );
                    }
                    OutboundDelegateMsg::SendDelegateMessage(_) => {
                        tracing::error!(
                            "SendDelegateMessage reached client serialization - this is a bug"
                        );
                    }
                });
                let messages_offset = builder.create_vector(&messages);
                let delegate_response_offset = FbsDelegateResponse::create(
                    &mut builder,
                    &DelegateResponseArgs {
                        key: Some(key_offset),
                        values: Some(messages_offset),
                    },
                );
                let host_response_offset = FbsHostResponse::create(
                    &mut builder,
                    &HostResponseArgs {
                        response_type: HostResponseType::DelegateResponse,
                        response: Some(delegate_response_offset.as_union_value()),
                    },
                );
                finish_host_response_buffer(&mut builder, host_response_offset);
                Ok(builder.finished_data().to_vec())
            }
            HostResponse::Ok => {
                let ok_offset = FbsOk::create(&mut builder, &OkArgs { msg: None });
                let host_response_offset = FbsHostResponse::create(
                    &mut builder,
                    &HostResponseArgs {
                        response_type: HostResponseType::Ok,
                        response: Some(ok_offset.as_union_value()),
                    },
                );
                finish_host_response_buffer(&mut builder, host_response_offset);
                Ok(builder.finished_data().to_vec())
            }
            HostResponse::QueryResponse(_) => unimplemented!(),
            HostResponse::StreamChunk {
                stream_id,
                index,
                total,
                data,
            } => {
                let data_offset = builder.create_vector(&data);
                let chunk_offset = FbsHostStreamChunk::create(
                    &mut builder,
                    &FbsHostStreamChunkArgs {
                        stream_id,
                        index,
                        total,
                        data: Some(data_offset),
                    },
                );
                let host_response_offset = FbsHostResponse::create(
                    &mut builder,
                    &HostResponseArgs {
                        response_type: HostResponseType::StreamChunk,
                        response: Some(chunk_offset.as_union_value()),
                    },
                );
                finish_host_response_buffer(&mut builder, host_response_offset);
                Ok(builder.finished_data().to_vec())
            }
            HostResponse::StreamHeader { .. } => {
                // StreamHeader is only sent over bincode (Native encoding) to
                // streaming-capable clients. Flatbuffers clients use transparent
                // reassembly via StreamChunk only.
                Err(Box::new(ClientError::from(ErrorKind::Unhandled {
                    cause: "StreamHeader is not supported over flatbuffers encoding".into(),
                })))
            }
        }
    }
}

impl Display for HostResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostResponse::ContractResponse(res) => match res {
                ContractResponse::PutResponse { key } => {
                    f.write_fmt(format_args!("put response for `{key}`"))
                }
                ContractResponse::UpdateResponse { key, .. } => {
                    f.write_fmt(format_args!("update response for `{key}`"))
                }
                ContractResponse::GetResponse { key, .. } => {
                    f.write_fmt(format_args!("get response for `{key}`"))
                }
                ContractResponse::UpdateNotification { key, .. } => {
                    f.write_fmt(format_args!("update notification for `{key}`"))
                }
                ContractResponse::SubscribeResponse { key, .. } => {
                    f.write_fmt(format_args!("subscribe response for `{key}`"))
                }
                ContractResponse::NotFound { instance_id } => {
                    f.write_fmt(format_args!("not found for `{instance_id}`"))
                }
            },
            HostResponse::DelegateResponse { .. } => write!(f, "delegate responses"),
            HostResponse::Ok => write!(f, "ok response"),
            HostResponse::QueryResponse(_) => write!(f, "query response"),
            HostResponse::StreamChunk {
                stream_id,
                index,
                total,
                ..
            } => write!(f, "stream chunk {index}/{total} (stream {stream_id})"),
            HostResponse::StreamHeader {
                stream_id,
                total_bytes,
                ..
            } => write!(f, "stream header (stream {stream_id}, {total_bytes} bytes)"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[non_exhaustive]
pub enum ContractResponse<T = WrappedState> {
    GetResponse {
        key: ContractKey,
        contract: Option<ContractContainer>,
        #[serde(bound(deserialize = "T: DeserializeOwned"))]
        state: T,
    },
    PutResponse {
        key: ContractKey,
    },
    /// Message sent when there is an update to a subscribed contract.
    UpdateNotification {
        key: ContractKey,
        #[serde(deserialize_with = "UpdateData::deser_update_data")]
        update: UpdateData<'static>,
    },
    /// Successful update
    UpdateResponse {
        key: ContractKey,
        #[serde(deserialize_with = "StateSummary::deser_state_summary")]
        summary: StateSummary<'static>,
    },
    SubscribeResponse {
        key: ContractKey,
        subscribed: bool,
    },
    /// Contract was not found after exhaustive search.
    /// This is an explicit response that distinguishes "contract doesn't exist"
    /// from other failure modes like timeouts or network errors.
    NotFound {
        /// The instance ID that was searched for.
        instance_id: ContractInstanceId,
    },
}

impl<T> From<ContractResponse<T>> for HostResponse<T> {
    fn from(value: ContractResponse<T>) -> HostResponse<T> {
        HostResponse::ContractResponse(value)
    }
}

#[cfg(test)]
mod node_diagnostics_response_tests {
    use super::{
        ConnectedPeerInfo, ContractState, NetworkInfo, NodeDiagnosticsResponse, NodeInfo,
        SubscriptionInfo, SystemMetrics,
    };
    use crate::contract_interface::ContractInstanceId;
    use std::collections::HashMap;

    /// Regression for freenet/freenet-core#3987.
    ///
    /// Pre-0.7 `contract_states` was `HashMap<ContractKey, ContractState>`.
    /// `ContractKey` derives `Serialize` as a struct (`{instance, code}`),
    /// which `serde_json` rejects with `key must be a string` because JSON
    /// object keys must be strings. The wire path between core and clients
    /// is bincode (which doesn't care about key types), so the bug stayed
    /// invisible until the `freenet service report` binary tried to
    /// JSON-serialize the response for upload — every report from a node
    /// hosting at least one contract uploaded with empty `network_status`.
    ///
    /// All six fields are populated so that any future `pub` field added
    /// to the struct gets exercised by serde_json the moment a contributor
    /// sets a non-default value here. If a future field reintroduces the
    /// non-string-key pattern (e.g. `HashMap<PeerId, _>`), this test will
    /// fail at the source instead of silently breaking the report path.
    #[test]
    fn node_diagnostics_response_json_round_trips() {
        let mut contract_states = HashMap::new();
        contract_states.insert(
            "6kVs66bKaQAC6ohr8b43SvJ95r36tc2hnG7HezmaJHF9".to_string(),
            ContractState {
                subscribers: 3,
                subscriber_peer_ids: vec!["peer-a".to_string(), "peer-b".to_string()],
                size_bytes: 1024,
            },
        );

        let response = NodeDiagnosticsResponse {
            node_info: Some(NodeInfo {
                peer_id: "peer-self".to_string(),
                is_gateway: true,
                location: Some("0.5".to_string()),
                listening_address: Some("0.0.0.0:31337".to_string()),
                uptime_seconds: 3600,
            }),
            network_info: Some(NetworkInfo {
                connected_peers: vec![("peer-x".to_string(), "10.0.0.1:31337".to_string())],
                active_connections: 1,
            }),
            subscriptions: vec![SubscriptionInfo {
                contract_key: ContractInstanceId::new([7u8; 32]),
                client_id: 42,
            }],
            contract_states,
            system_metrics: Some(SystemMetrics {
                active_connections: 1,
                hosting_contracts: 1,
            }),
            connected_peers_detailed: vec![ConnectedPeerInfo {
                peer_id: "peer-x".to_string(),
                address: "10.0.0.1:31337".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).expect("must serialize to JSON");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("output is valid JSON");

        // Every top-level field is present and distinguishable.
        let obj = parsed.as_object().expect("top-level must be object");
        assert_eq!(obj.len(), 6, "expected six top-level fields, got {obj:?}");
        assert_eq!(parsed["node_info"]["peer_id"], "peer-self");
        assert_eq!(parsed["network_info"]["active_connections"], 1);
        assert_eq!(parsed["subscriptions"][0]["client_id"], 42);
        assert_eq!(parsed["system_metrics"]["hosting_contracts"], 1);
        assert_eq!(parsed["connected_peers_detailed"][0]["peer_id"], "peer-x");

        let states = parsed["contract_states"]
            .as_object()
            .expect("contract_states must be a JSON object");
        assert_eq!(states.len(), 1);
        assert_eq!(
            states["6kVs66bKaQAC6ohr8b43SvJ95r36tc2hnG7HezmaJHF9"]["subscribers"],
            3
        );

        // Bincode round-trip must also still work for the same value
        // (the new wire format is the contract; older clients are
        // documented as incompatible in CHANGELOG).
        let bytes = bincode::serialize(&response).expect("bincode must serialize");
        let decoded: NodeDiagnosticsResponse =
            bincode::deserialize(&bytes).expect("bincode must round-trip");
        assert_eq!(
            decoded.contract_states.len(),
            1,
            "bincode round-trip preserves contract_states entries"
        );
    }
}

#[cfg(test)]
mod client_request_test {
    use crate::client_api::{ContractRequest, TryFromFbs, WsApiError};
    use crate::contract_interface::UpdateData;
    use crate::generated::client_request::root_as_client_request;

    const EXPECTED_ENCODED_CONTRACT_ID: &str = "6kVs66bKaQAC6ohr8b43SvJ95r36tc2hnG7HezmaJHF9";

    #[test]
    fn test_build_contract_put_op_from_fbs() -> Result<(), Box<dyn std::error::Error>> {
        let put_req_op = vec![
            4, 0, 0, 0, 244, 255, 255, 255, 16, 0, 0, 0, 0, 0, 0, 1, 8, 0, 12, 0, 11, 0, 4, 0, 8,
            0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 1, 198, 255, 255, 255, 12, 0, 0, 0, 20, 0, 0, 0, 36, 0,
            0, 0, 170, 255, 255, 255, 4, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
            8, 0, 10, 0, 9, 0, 4, 0, 8, 0, 0, 0, 16, 0, 0, 0, 0, 1, 10, 0, 16, 0, 12, 0, 8, 0, 4,
            0, 10, 0, 0, 0, 12, 0, 0, 0, 76, 0, 0, 0, 92, 0, 0, 0, 176, 255, 255, 255, 8, 0, 0, 0,
            16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 8, 0, 4, 0, 6, 0, 0, 0, 4, 0, 0, 0, 32, 0, 0, 0,
            85, 111, 11, 171, 40, 85, 240, 177, 207, 81, 106, 157, 173, 90, 234, 2, 250, 253, 75,
            210, 62, 7, 6, 34, 75, 26, 229, 230, 107, 167, 17, 108, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6,
            7, 8, 8, 0, 12, 0, 8, 0, 4, 0, 8, 0, 0, 0, 8, 0, 0, 0, 16, 0, 0, 0, 8, 0, 0, 0, 1, 2,
            3, 4, 5, 6, 7, 8, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
        ];
        let request = if let Ok(client_request) = root_as_client_request(&put_req_op) {
            let contract_request = client_request.client_request_as_contract_request().unwrap();
            ContractRequest::try_decode_fbs(&contract_request)?
        } else {
            panic!("failed to decode client request")
        };

        match request {
            ContractRequest::Put {
                contract,
                state,
                related_contracts: _,
                subscribe,
                blocking_subscribe,
            } => {
                assert_eq!(
                    contract.to_string(),
                    "WasmContainer([api=0.0.1](D8fdVLbRyMLw5mZtPRpWMFcrXGN2z8Nq8UGcLGPFBg2W))"
                );
                assert_eq!(contract.unwrap_v1().data.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
                assert_eq!(state.to_vec(), &[1, 2, 3, 4, 5, 6, 7, 8]);
                assert!(!subscribe);
                assert!(!blocking_subscribe);
            }
            _ => panic!("wrong contract request type"),
        }

        Ok(())
    }

    #[test]
    fn test_build_contract_get_op_from_fbs() -> Result<(), Box<dyn std::error::Error>> {
        let get_req_op = vec![
            4, 0, 0, 0, 244, 255, 255, 255, 16, 0, 0, 0, 0, 0, 0, 1, 8, 0, 12, 0, 11, 0, 4, 0, 8,
            0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 3, 222, 255, 255, 255, 12, 0, 0, 0, 8, 0, 12, 0, 8, 0, 4,
            0, 8, 0, 0, 0, 8, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 8, 0, 4, 0, 6, 0, 0, 0,
            4, 0, 0, 0, 32, 0, 0, 0, 85, 111, 11, 171, 40, 85, 240, 177, 207, 81, 106, 157, 173,
            90, 234, 2, 250, 253, 75, 210, 62, 7, 6, 34, 75, 26, 229, 230, 107, 167, 17, 108,
        ];
        let request = if let Ok(client_request) = root_as_client_request(&get_req_op) {
            let contract_request = client_request.client_request_as_contract_request().unwrap();
            ContractRequest::try_decode_fbs(&contract_request)?
        } else {
            panic!("failed to decode client request")
        };

        match request {
            ContractRequest::Get {
                key,
                return_contract_code: fetch_contract,
                subscribe,
                blocking_subscribe,
            } => {
                assert_eq!(key.encode(), EXPECTED_ENCODED_CONTRACT_ID);
                assert!(!fetch_contract);
                assert!(!subscribe);
                assert!(!blocking_subscribe);
            }
            _ => panic!("wrong contract request type"),
        }

        Ok(())
    }

    /// A well-formed FlatBuffers `UpdateRequest` decodes to the expected key
    /// and delta.
    ///
    /// Built programmatically rather than from a hardcoded byte blob. The blob
    /// this replaces carried a present-but-ZERO-LENGTH `code` field, which is
    /// what the TypeScript SDK's `ContractKey.fromInstanceId(...)` emits: a
    /// request the node cannot serve, because an UPDATE supplies no contract
    /// code and freenet-core resolves the WASM by code hash. Nobody could see
    /// that from reading the test; it took instrumenting the decoder. So the
    /// fixture now carries a real 32-byte hash and says so in source.
    ///
    /// The empty and absent cases are covered in
    /// `contract_interface::key::fbs_tests`, next to the decoder that rejects
    /// them.
    #[test]
    fn test_build_contract_update_op_from_fbs() -> Result<(), Box<dyn std::error::Error>> {
        use crate::generated::client_request::{
            finish_client_request_buffer, ClientRequest as FbsClientRequest, ClientRequestArgs,
            ClientRequestType, ContractRequest as FbsContractRequest, ContractRequestArgs,
            ContractRequestType, Update as FbsUpdate, UpdateArgs,
        };
        use crate::generated::common::{
            ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs,
            ContractKey as FbsContractKey, ContractKeyArgs, DeltaUpdate, DeltaUpdateArgs,
            UpdateData as FbsUpdateData, UpdateDataArgs, UpdateDataType,
        };
        use crate::prelude::ContractInstanceId;

        // Derive the instance bytes from the expected id rather than pasting a
        // magic array, so the assertion below can't drift from the fixture.
        let instance_id = ContractInstanceId::try_from(EXPECTED_ENCODED_CONTRACT_ID.to_string())?;
        // A distinct, arbitrary code hash: if the decoder ever re-hashes it
        // (the BLAKE3(BLAKE3(wasm)) bug this PR fixes), the assertion fails.
        let code_hash = [42u8; 32];
        let delta_bytes = [1u8, 2, 3, 4, 5, 6, 7, 8];

        let mut b = flatbuffers::FlatBufferBuilder::new();

        let instance_data = b.create_vector(instance_id.as_bytes());
        let instance_offset = FbsContractInstanceId::create(
            &mut b,
            &ContractInstanceIdArgs {
                data: Some(instance_data),
            },
        );
        let code = Some(b.create_vector(&code_hash));
        let key_offset = FbsContractKey::create(
            &mut b,
            &ContractKeyArgs {
                instance: Some(instance_offset),
                code,
            },
        );

        let delta = b.create_vector(&delta_bytes);
        let delta_offset = DeltaUpdate::create(&mut b, &DeltaUpdateArgs { delta: Some(delta) });
        let update_data_offset = FbsUpdateData::create(
            &mut b,
            &UpdateDataArgs {
                update_data_type: UpdateDataType::DeltaUpdate,
                update_data: Some(delta_offset.as_union_value()),
            },
        );

        let update_offset = FbsUpdate::create(
            &mut b,
            &UpdateArgs {
                key: Some(key_offset),
                data: Some(update_data_offset),
            },
        );
        let contract_offset = FbsContractRequest::create(
            &mut b,
            &ContractRequestArgs {
                contract_request_type: ContractRequestType::Update,
                contract_request: Some(update_offset.as_union_value()),
            },
        );
        let client_offset = FbsClientRequest::create(
            &mut b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::ContractRequest,
                client_request: Some(contract_offset.as_union_value()),
            },
        );
        finish_client_request_buffer(&mut b, client_offset);

        let update_op = b.finished_data().to_vec();
        let request = if let Ok(client_request) = root_as_client_request(&update_op) {
            let contract_request = client_request.client_request_as_contract_request().unwrap();
            ContractRequest::try_decode_fbs(&contract_request)?
        } else {
            panic!("failed to decode client request")
        };

        match request {
            ContractRequest::Update { key, data } => {
                assert_eq!(key.encoded_contract_id(), EXPECTED_ENCODED_CONTRACT_ID);
                assert_eq!(
                    key.code_hash().as_ref(),
                    &code_hash,
                    "the code hash must survive decode unchanged, not be re-hashed"
                );
                match data {
                    UpdateData::Delta(delta) => {
                        assert_eq!(delta.to_vec(), &delta_bytes)
                    }
                    _ => panic!("wrong update data type"),
                }
            }
            _ => panic!("wrong contract request type"),
        }

        Ok(())
    }

    /// The exact bytes the TypeScript SDK's own test suite asserts as a correct
    /// `UpdateRequest`: pinned here so the two suites cannot disagree silently.
    ///
    /// Copied verbatim from `typescript/tests/websocket-interface.test.ts`
    /// (`EXPECTED_UPDATE_REQ`). It is byte-for-byte the blob this PR deleted
    /// from the Rust side, which is exactly how the original double-hash bug
    /// survived: both suites pinned the same bytes in mirror, and neither ever
    /// crossed the language boundary, so a shape that no Rust decoder accepted
    /// stayed "verified" on the TypeScript side.
    ///
    /// Its `code` field is present and ZERO-LENGTH, which is what
    /// `ContractKey.fromInstanceId(...)` emits. After this change the Rust
    /// decoder hard-errors on it, while `npm test` still asserts it is correct
    /// and stays green. Both suites run in CI, so this test is the only thing
    /// that makes that disagreement visible.
    ///
    /// Scope, so this is not over-trusted: it is a COPY of the TypeScript
    /// array, not a reference to it. Nothing mechanically ties the two, so
    /// editing the TypeScript side fails nothing here. (`include_str!` across
    /// to `typescript/` is not the fix: `rust/` is the published package root
    /// and the path would escape it.) So this is a convention, and the
    /// convention is: if the TypeScript SDK is fixed (see
    /// freenet/freenet-core#4978, whose candidate is making `UpdateRequest`
    /// reject a key whose `code` is not 32 bytes), update the array and this
    /// constant together. Leaving one behind re-creates the mirror-pinning
    /// that hid the original bug.
    const TS_SDK_EXPECTED_UPDATE_REQ: &[u8] = &[
        4, 0, 0, 0, 220, 255, 255, 255, 8, 0, 0, 0, 0, 0, 0, 1, 232, 255, 255, 255, 8, 0, 0, 0, 0,
        0, 0, 2, 204, 255, 255, 255, 16, 0, 0, 0, 52, 0, 0, 0, 8, 0, 12, 0, 11, 0, 4, 0, 8, 0, 0,
        0, 8, 0, 0, 0, 0, 0, 0, 2, 210, 255, 255, 255, 4, 0, 0, 0, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7,
        8, 8, 0, 12, 0, 8, 0, 4, 0, 8, 0, 0, 0, 8, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 8,
        0, 4, 0, 6, 0, 0, 0, 4, 0, 0, 0, 32, 0, 0, 0, 85, 111, 11, 171, 40, 85, 240, 177, 207, 81,
        106, 157, 173, 90, 234, 2, 250, 253, 75, 210, 62, 7, 6, 34, 75, 26, 229, 230, 107, 167, 17,
        108,
    ];

    /// The cross-language contract: what the TypeScript SDK emits for an
    /// instance-id-only UPDATE is rejected here, with the actionable message.
    ///
    /// Asserting the MESSAGE and not just the rejection is the point. The
    /// TypeScript developer whose request this is gets exactly this string
    /// back over the WebSocket, and it is the only thing telling them the key
    /// needs both parts. A revert to the bare `try_from` error would leave the
    /// rejection green while restoring `"invalid data"`.
    #[test]
    fn typescript_sdk_instance_only_update_is_rejected_with_guidance() {
        let client_request = root_as_client_request(TS_SDK_EXPECTED_UPDATE_REQ)
            .expect("the TS SDK blob must still be a well-formed ClientRequest");
        let contract_request = client_request
            .client_request_as_contract_request()
            .expect("the TS SDK blob must still be a ContractRequest");

        let err = ContractRequest::try_decode_fbs(&contract_request)
            .expect_err("an instance-id-only UPDATE must be rejected");
        let msg = err.to_string();

        assert!(
            msg.contains("ContractKey.code") && msg.contains("got 0 bytes"),
            "the TS SDK's zero-length code must be named explicitly, got: {msg}"
        );
        assert!(
            msg.contains("new ContractKey(instance, code)"),
            "the error must tell a TypeScript developer how to build the key, got: {msg}"
        );
        assert!(
            msg.contains("4978"),
            "the error must point at the tracking issue for the real fix, got: {msg}"
        );
    }

    /// Build a `ClientRequest` carrying a GET or SUBSCRIBE whose `ContractKey`
    /// has an `instance` vector of the given length.
    ///
    /// The length is a parameter because the flatbuffers verifier checks that a
    /// `(required)` vector is PRESENT, not that it is the right SIZE, so a
    /// wrong-length instance is a shape a peer can actually put on the wire and
    /// that `flatbuffers::root` will happily hand to the decoder.
    fn client_request_with_instance_len(instance_len: usize, subscribe: bool) -> Vec<u8> {
        use crate::generated::client_request::{
            finish_client_request_buffer, ClientRequest as FbsClientRequest, ClientRequestArgs,
            ClientRequestType, ContractRequest as FbsContractRequest, ContractRequestArgs,
            ContractRequestType, Get as FbsGet, GetArgs, Subscribe as FbsSubscribe, SubscribeArgs,
        };
        use crate::generated::common::{
            ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs,
            ContractKey as FbsContractKey, ContractKeyArgs,
        };

        let mut b = flatbuffers::FlatBufferBuilder::new();
        let instance_data = b.create_vector(&vec![1u8; instance_len]);
        let instance_offset = FbsContractInstanceId::create(
            &mut b,
            &ContractInstanceIdArgs {
                data: Some(instance_data),
            },
        );
        let code = Some(b.create_vector(&[42u8; 32]));
        let key_offset = FbsContractKey::create(
            &mut b,
            &ContractKeyArgs {
                instance: Some(instance_offset),
                code,
            },
        );

        let (request_type, request_offset) = if subscribe {
            let sub = FbsSubscribe::create(
                &mut b,
                &SubscribeArgs {
                    key: Some(key_offset),
                    summary: None,
                },
            );
            (ContractRequestType::Subscribe, sub.as_union_value())
        } else {
            let get = FbsGet::create(
                &mut b,
                &GetArgs {
                    key: Some(key_offset),
                    fetch_contract: false,
                    subscribe: false,
                    blocking_subscribe: false,
                },
            );
            (ContractRequestType::Get, get.as_union_value())
        };

        let contract_offset = FbsContractRequest::create(
            &mut b,
            &ContractRequestArgs {
                contract_request_type: request_type,
                contract_request: Some(request_offset),
            },
        );
        let client_offset = FbsClientRequest::create(
            &mut b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::ContractRequest,
                client_request: Some(contract_offset.as_union_value()),
            },
        );
        finish_client_request_buffer(&mut b, client_offset);
        b.finished_data().to_vec()
    }

    fn decode_client_request(bytes: &[u8]) -> Result<ContractRequest<'_>, WsApiError> {
        let client_request =
            root_as_client_request(bytes).expect("must be a well-formed ClientRequest");
        let contract_request = client_request
            .client_request_as_contract_request()
            .expect("must be a ContractRequest");
        ContractRequest::try_decode_fbs(&contract_request)
    }

    /// A GET whose instance is the wrong length is rejected, not panicked on.
    ///
    /// Pinned at the REQUEST level rather than through `ContractKey`'s decoder,
    /// because GET does not go through `ContractKey::try_decode_fbs` at all: it
    /// reads the instance bytes directly. A test that only covers the UPDATE
    /// path leaves a revert of this site green, which is exactly the state this
    /// suite was in before: the panic fix reached three call sites and only one
    /// of them was pinned.
    #[test]
    fn get_with_wrong_length_instance_is_rejected_not_panicking() {
        let bytes = client_request_with_instance_len(8, false);
        let short = decode_client_request(&bytes)
            .expect_err("a GET with an 8-byte instance must be rejected");
        assert!(
            short.to_string().contains("ContractKey.instance")
                && short.to_string().contains("got 8 bytes"),
            "got: {short}"
        );

        let bytes = client_request_with_instance_len(64, false);
        let long = decode_client_request(&bytes)
            .expect_err("a GET with a 64-byte instance must be rejected");
        assert!(long.to_string().contains("got 64 bytes"), "got: {long}");
    }

    /// Same for SUBSCRIBE, which is a third independent decode site.
    #[test]
    fn subscribe_with_wrong_length_instance_is_rejected_not_panicking() {
        let bytes = client_request_with_instance_len(8, true);
        let short = decode_client_request(&bytes)
            .expect_err("a SUBSCRIBE with an 8-byte instance must be rejected");
        assert!(
            short.to_string().contains("ContractKey.instance")
                && short.to_string().contains("got 8 bytes"),
            "got: {short}"
        );

        let bytes = client_request_with_instance_len(64, true);
        let long = decode_client_request(&bytes)
            .expect_err("a SUBSCRIBE with a 64-byte instance must be rejected");
        assert!(long.to_string().contains("got 64 bytes"), "got: {long}");
    }

    /// A well-formed GET still decodes: the length guard must reject the wrong
    /// size without breaking the right one.
    #[test]
    fn get_with_valid_instance_still_decodes() {
        let bytes = client_request_with_instance_len(32, false);
        let req = decode_client_request(&bytes).expect("a 32-byte instance must still decode");
        assert!(
            matches!(req, ContractRequest::Get { .. }),
            "expected a Get, got {req:?}"
        );
    }

    /// The flatbuffers decode path must NOT panic on an unknown
    /// `ContractRequestType` discriminant. Same class as the `DelegateRequest`
    /// guard: the generated union verifier accepts any discriminant it doesn't
    /// recognize (`_ => Ok(())`), and the union type field is a raw `u8` a
    /// client can set to any value. Before the fix this hit `unreachable!()`
    /// and downed the connection handler; now it is a clean per-request error.
    #[test]
    fn fbs_decode_rejects_unknown_contract_discriminant() {
        use crate::generated::client_request::{
            finish_client_request_buffer, ClientRequest as FbsClientRequest, ClientRequestArgs,
            ClientRequestType, ContractRequest as FbsContractRequest, ContractRequestArgs,
            ContractRequestType, DelegateKey as FbsDelegateKey, DelegateKeyArgs,
            UnregisterDelegate, UnregisterDelegateArgs,
        };

        let mut b = flatbuffers::FlatBufferBuilder::new();
        // Any well-formed table satisfies the required union value; the
        // discriminant is unknown so the verifier never inspects it.
        let key = b.create_vector(&[0u8; 32]);
        let code_hash = b.create_vector(&[0u8; 32]);
        let dk = FbsDelegateKey::create(
            &mut b,
            &DelegateKeyArgs {
                key: Some(key),
                code_hash: Some(code_hash),
            },
        );
        let dummy = UnregisterDelegate::create(&mut b, &UnregisterDelegateArgs { key: Some(dk) });
        // 99 is past the max known ContractRequestType (Subscribe = 4).
        let contract = FbsContractRequest::create(
            &mut b,
            &ContractRequestArgs {
                contract_request_type: ContractRequestType(99),
                contract_request: Some(dummy.as_union_value()),
            },
        );
        let client = FbsClientRequest::create(
            &mut b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::ContractRequest,
                client_request: Some(contract.as_union_value()),
            },
        );
        finish_client_request_buffer(&mut b, client);
        let bytes = b.finished_data().to_vec();

        let client =
            root_as_client_request(&bytes).expect("verifier accepts an unknown union discriminant");
        let fbs_contract = client
            .client_request_as_contract_request()
            .expect("client_request is a ContractRequest");
        assert!(
            ContractRequest::try_decode_fbs(&fbs_contract).is_err(),
            "an unknown ContractRequestType discriminant must be a clean \
             per-request error, never a panic that downs the connection handler"
        );
    }
}

/// Wire-format pins for [`DelegateRequest`].
///
/// `DelegateRequest` crosses the client<->node boundary as bincode (the
/// `EncodingProtocol::Native` path in freenet-core; the Rust clients in this
/// crate — `browser.rs` and `regular.rs` — both `bincode::serialize` their
/// requests). Bincode encodes an enum's variant as a 4-byte little-endian
/// `u32` discriminant, so the *order* of the variants is the wire contract:
/// reordering or inserting a variant anywhere but the end silently reassigns
/// every following tag and breaks already-deployed clients (the v0.2.11
/// break class).
///
/// These tests exist because, before this module, `DelegateRequest` had NO
/// wire-format pin at all — a reorder would have shipped undetected.
///
/// A fourth variant, `RegisterDelegateWithPredecessors` (tag 3), was added in
/// 0.8.4 and removed here in 0.9.0 (freenet-core#5199, tracking issue
/// freenet-core#5198 — its `origin_contract` authorization gate was forgeable
/// by any HTTP client, and no client ever constructed or sent it on the
/// wire). It was appended last specifically so removing it leaves tags 0-2
/// unaffected; the three variants below are exactly what shipped from the
/// start, still pinned to their complete byte encodings. Tag 3 is free to
/// reuse for a future variant: since nothing ever spoke it on the wire,
/// nothing can misinterpret it.
#[cfg(test)]
mod delegate_request_wire_format {
    use super::DelegateRequest;
    use crate::code_hash::CodeHash;
    use crate::prelude::{
        ApplicationMessage, Delegate, DelegateCode, DelegateContainer, DelegateKey,
        DelegateWasmAPIVersion, InboundDelegateMsg, Parameters,
    };

    fn sample_container() -> DelegateContainer {
        let code = DelegateCode::from(vec![1u8, 2, 3, 4]);
        let params = Parameters::from(vec![9u8, 8, 7]);
        DelegateContainer::Wasm(DelegateWasmAPIVersion::V1(Delegate::from((&code, &params))))
    }

    // The four sample values whose complete bincode encodings are frozen in
    // `wire_format_is_frozen`. Kept byte-for-byte identical to the throwaway
    // generator that produced the frozen vectors, so the freeze is
    // reproducible: construct the value, `bincode::serialize`, compare.
    fn sample_app_messages() -> DelegateRequest<'static> {
        DelegateRequest::ApplicationMessages {
            key: DelegateKey::new([0x11; 32], CodeHash::new([0x22; 32])),
            params: Parameters::from(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            inbound: vec![InboundDelegateMsg::ApplicationMessage(
                ApplicationMessage::new(vec![0x01, 0x02, 0x03]),
            )],
        }
    }

    fn sample_register() -> DelegateRequest<'static> {
        DelegateRequest::RegisterDelegate {
            delegate: sample_container(),
            cipher: [0x55; 32],
            nonce: [0x66; 24],
        }
    }

    fn sample_unregister() -> DelegateRequest<'static> {
        DelegateRequest::UnregisterDelegate(DelegateKey::new([0x11; 32], CodeHash::new([0x22; 32])))
    }

    /// Complete-byte wire-format freeze for all three variants.
    ///
    /// Each variant (`ApplicationMessages`, `RegisterDelegate`,
    /// `UnregisterDelegate`) is pinned to its FULL expected byte vector — not
    /// just the 4-byte tag — so a field reorder or a change to a nested
    /// type's encoding (e.g. `DelegateContainer`, `DelegateKey`,
    /// `ApplicationMessage`) is caught, not only a variant reorder. These
    /// vectors were generated from **origin/main @ 8b53702** (the shipped
    /// format) via a throwaway generator and confirmed byte-identical to this
    /// branch's output, so the pin anchors to what old clients actually
    /// speak.
    ///
    /// The test also DESERIALIZES each frozen vector, proving an old-format
    /// byte stream still decodes into the expected variant on this build.
    #[test]
    fn wire_format_is_frozen() {
        // --- origin/main @ 8b53702 (shipped format) ---
        const APP_MESSAGES: &[u8] = &[
            0, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
            17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 4, 0, 0, 0, 0, 0, 0, 0, 222, 173, 190, 239, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
            0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        const REGISTER: &[u8] = &[
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7, 4, 0, 0, 0, 0, 0,
            0, 0, 1, 2, 3, 4, 99, 120, 29, 23, 20, 37, 163, 99, 18, 250, 5, 141, 135, 18, 213, 208,
            81, 53, 169, 145, 236, 32, 53, 28, 233, 214, 92, 219, 25, 160, 84, 50, 88, 111, 44, 39,
            24, 219, 97, 92, 222, 20, 205, 248, 149, 154, 214, 38, 193, 144, 31, 141, 32, 222, 49,
            197, 66, 237, 16, 98, 165, 72, 6, 11, 99, 120, 29, 23, 20, 37, 163, 99, 18, 250, 5,
            141, 135, 18, 213, 208, 81, 53, 169, 145, 236, 32, 53, 28, 233, 214, 92, 219, 25, 160,
            84, 50, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85,
            85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 102, 102, 102, 102, 102, 102, 102, 102,
            102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102,
        ];
        const UNREGISTER: &[u8] = &[
            2, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
            17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34,
        ];
        // 1. Serialization is byte-stable for every variant.
        assert_eq!(
            bincode::serialize(&sample_app_messages()).unwrap(),
            APP_MESSAGES,
            "ApplicationMessages (tag 0) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_register()).unwrap(),
            REGISTER,
            "RegisterDelegate (tag 1) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_unregister()).unwrap(),
            UNREGISTER,
            "UnregisterDelegate (tag 2) encoding changed"
        );

        // 2. The three variant tags are exactly 0,1,2.
        assert_eq!(APP_MESSAGES[..4], 0u32.to_le_bytes());
        assert_eq!(REGISTER[..4], 1u32.to_le_bytes());
        assert_eq!(UNREGISTER[..4], 2u32.to_le_bytes());

        // 3. Each frozen (old-format) byte stream still DECODES into its
        //    variant on this build — an old client's bytes remain readable.
        assert!(matches!(
            bincode::deserialize::<DelegateRequest>(APP_MESSAGES).unwrap(),
            DelegateRequest::ApplicationMessages { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<DelegateRequest>(REGISTER).unwrap(),
            DelegateRequest::RegisterDelegate { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<DelegateRequest>(UNREGISTER).unwrap(),
            DelegateRequest::UnregisterDelegate(_)
        ));
    }

    /// `key()` dispatches correctly for every remaining variant: the
    /// `ApplicationMessages`/`UnregisterDelegate` key field directly, and
    /// `RegisterDelegate` via the contained delegate's own key.
    #[test]
    fn key_dispatches_for_every_variant() {
        let app_key = DelegateKey::new([0x11; 32], CodeHash::new([0x22; 32]));
        assert_eq!(sample_app_messages().key(), &app_key);

        let register = sample_register();
        match &register {
            DelegateRequest::RegisterDelegate { delegate, .. } => {
                assert_eq!(register.key(), delegate.key());
            }
            other => panic!("sample_register() must build a RegisterDelegate, got {other:?}"),
        }

        let unregister_key = DelegateKey::new([0x11; 32], CodeHash::new([0x22; 32]));
        assert_eq!(sample_unregister().key(), &unregister_key);
    }

    /// The flatbuffers decode path must NOT panic on an unknown
    /// `DelegateRequestType` discriminant. The generated union verifier accepts
    /// any discriminant it doesn't recognize (`_ => Ok(())`), and the union
    /// type field is a raw `u8` that the public (TypeScript) builder can set to
    /// any value, so a crafted request reaches `DelegateRequest::try_decode_fbs`
    /// with an out-of-range discriminant. Before the fix this hit
    /// `unreachable!()` and took down the connection handler; now it is a clean
    /// per-request error. (Regression guard for the P2 finding on PR #86.)
    #[test]
    fn fbs_decode_rejects_unknown_discriminant() {
        use crate::client_api::TryFromFbs;
        use crate::generated::client_request::{
            finish_client_request_buffer, root_as_client_request,
            ClientRequest as FbsClientRequest, ClientRequestArgs, ClientRequestType,
            DelegateKey as FbsDelegateKey, DelegateKeyArgs, DelegateRequest as FbsDelegateRequest,
            DelegateRequestArgs, DelegateRequestType, UnregisterDelegate, UnregisterDelegateArgs,
        };

        let mut b = flatbuffers::FlatBufferBuilder::new();
        // A real, well-formed UnregisterDelegate table to hang off the union...
        let key = b.create_vector(&[0u8; 32]);
        let code_hash = b.create_vector(&[0u8; 32]);
        let dk = FbsDelegateKey::create(
            &mut b,
            &DelegateKeyArgs {
                key: Some(key),
                code_hash: Some(code_hash),
            },
        );
        let unreg = UnregisterDelegate::create(&mut b, &UnregisterDelegateArgs { key: Some(dk) });
        // ...but LIE about the union type: 99 is past the max known
        // discriminant (UnregisterDelegate = 3), yet the verifier accepts it.
        let dreq = FbsDelegateRequest::create(
            &mut b,
            &DelegateRequestArgs {
                delegate_request_type: DelegateRequestType(99),
                delegate_request: Some(unreg.as_union_value()),
            },
        );
        let creq = FbsClientRequest::create(
            &mut b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::DelegateRequest,
                client_request: Some(dreq.as_union_value()),
            },
        );
        finish_client_request_buffer(&mut b, creq);
        let bytes = b.finished_data().to_vec();

        let client =
            root_as_client_request(&bytes).expect("verifier accepts an unknown union discriminant");
        let fbs_delegate = client
            .client_request_as_delegate_request()
            .expect("client_request is a DelegateRequest");
        let decoded = DelegateRequest::try_decode_fbs(&fbs_delegate);
        assert!(
            decoded.is_err(),
            "an unknown DelegateRequestType discriminant must be a clean \
             per-request error, never a panic that downs the connection handler"
        );
    }
}

/// Wire-format pins for [`ContractRequest`].
///
/// Same rationale as `delegate_request_wire_format` above: `ContractRequest`
/// crosses the client<->node boundary as bincode (the `EncodingProtocol::Native`
/// path; `browser.rs` and `regular.rs` both `bincode::serialize` their
/// requests), which encodes an enum's variant as a 4-byte little-endian `u32`
/// discriminant. The *declaration order* of the variants is therefore part of
/// the wire contract: reordering or inserting a variant anywhere but the end
/// silently reassigns every following tag and breaks already-deployed clients
/// (the v0.2.11 break class). `ContractRequest` had no such pin before this
/// module, and a new `Unsubscribe` variant is about to be appended — this
/// closes the gap first.
///
/// Each variant (`Put`, `Update`, `Get`, `Subscribe`) is pinned to its FULL
/// expected byte vector, not just the 4-byte tag, so a field reorder or a
/// change to a nested type's encoding (`ContractContainer`, `ContractKey`,
/// `UpdateData`, ...) is caught too. Unlike the `DelegateRequest` freeze,
/// there is no earlier shipped commit to anchor these bytes to — this module
/// establishes the frozen baseline from the current encoding; from here on,
/// any change to these bytes must be deliberate and version-gated.
///
/// Also pinned: the flatbuffers `ContractRequestType` union discriminants
/// (`schemas/flatbuffers/client_request.fbs`). Those are fixed by the
/// schema's declaration order the same way bincode's are fixed by the Rust
/// enum's declaration order, so a schema reorder is just as much a wire
/// break for flatbuffers/browser clients as a Rust reorder is for native
/// ones.
#[cfg(test)]
mod contract_request_wire_format {
    use super::ContractRequest;
    use crate::code_hash::CodeHash;
    use crate::generated::client_request::ContractRequestType;
    use crate::prelude::{
        ContractCode, ContractContainer, ContractInstanceId, ContractKey, ContractWasmAPIVersion,
        Parameters, RelatedContracts, State, StateSummary, UpdateData, WrappedContract,
        WrappedState,
    };
    use std::sync::Arc;

    fn sample_key() -> ContractKey {
        ContractKey::from_id_and_code(
            ContractInstanceId::new([0x11; 32]),
            CodeHash::new([0x22; 32]),
        )
    }

    fn sample_container() -> ContractContainer {
        let code = Arc::new(ContractCode::from(vec![1u8, 2, 3]));
        let params = Parameters::from(vec![9u8, 8, 7]);
        ContractContainer::Wasm(ContractWasmAPIVersion::V1(WrappedContract::new(
            code, params,
        )))
    }

    // The four sample values whose complete bincode encodings are frozen in
    // `wire_format_is_frozen`. Kept byte-for-byte identical to the throwaway
    // generator that produced the frozen vectors, so the freeze is
    // reproducible: construct the value, `bincode::serialize`, compare.
    fn sample_put() -> ContractRequest<'static> {
        ContractRequest::Put {
            contract: sample_container(),
            state: WrappedState::new(vec![0x44, 0x55, 0x66]),
            related_contracts: RelatedContracts::new(),
            subscribe: true,
            blocking_subscribe: false,
        }
    }

    fn sample_update() -> ContractRequest<'static> {
        ContractRequest::Update {
            key: sample_key(),
            data: UpdateData::State(State::from(vec![0x77, 0x88])),
        }
    }

    fn sample_get() -> ContractRequest<'static> {
        ContractRequest::Get {
            key: ContractInstanceId::new([0x33; 32]),
            return_contract_code: true,
            subscribe: false,
            blocking_subscribe: false,
        }
    }

    /// Complements [`sample_get`], whose three consecutive `bool` fields cannot
    /// all be pairwise distinguishable in a single sample (three bools always
    /// share a value somewhere). `sample_get` sets `subscribe` and
    /// `blocking_subscribe` both false, so a declaration-order swap of those two
    /// encodes identically and its frozen vector alone cannot catch it. This
    /// sample differs in exactly that pair, so together the two freeze every
    /// pairwise field order in `Get`.
    fn sample_get_subscribe_pair() -> ContractRequest<'static> {
        ContractRequest::Get {
            key: ContractInstanceId::new([0x33; 32]),
            return_contract_code: false,
            subscribe: true,
            blocking_subscribe: false,
        }
    }

    fn sample_subscribe() -> ContractRequest<'static> {
        ContractRequest::Subscribe {
            key: ContractInstanceId::new([0x33; 32]),
            summary: Some(StateSummary::from(vec![0x99, 0xAA])),
        }
    }

    /// Complete-byte wire-format freeze for all four variants.
    ///
    /// The test also DESERIALIZES each frozen vector, proving the byte
    /// stream still decodes into the expected variant on this build.
    #[test]
    fn wire_format_is_frozen() {
        const PUT: &[u8] = &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 177, 119, 236, 27,
            242, 109, 251, 59, 112, 16, 212, 115, 230, 212, 71, 19, 178, 155, 118, 91, 153, 198,
            230, 14, 203, 250, 231, 66, 222, 73, 101, 67, 3, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7, 68, 0,
            48, 164, 43, 234, 3, 4, 34, 33, 221, 91, 193, 53, 159, 47, 206, 127, 237, 159, 116, 81,
            44, 75, 126, 103, 73, 141, 96, 191, 52, 206, 177, 119, 236, 27, 242, 109, 251, 59, 112,
            16, 212, 115, 230, 212, 71, 19, 178, 155, 118, 91, 153, 198, 230, 14, 203, 250, 231,
            66, 222, 73, 101, 67, 3, 0, 0, 0, 0, 0, 0, 0, 68, 85, 102, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0,
        ];
        const UPDATE: &[u8] = &[
            1, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
            17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 119, 136,
        ];
        const GET: &[u8] = &[
            2, 0, 0, 0, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51,
            51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 1, 0, 0,
        ];
        // Complements GET: `sample_get` leaves `subscribe` and
        // `blocking_subscribe` both false, so a declaration-order swap of that
        // pair encodes identically and GET alone cannot catch it. Three bools
        // can never all be pairwise distinct in one sample, so a second vector
        // differing in exactly that pair is what completes the freeze.
        const GET_SUBSCRIBE_PAIR: &[u8] = &[
            2, 0, 0, 0, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51,
            51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 0, 1, 0,
        ];
        const SUBSCRIBE: &[u8] = &[
            3, 0, 0, 0, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51,
            51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 1, 2, 0, 0, 0, 0, 0, 0, 0, 153,
            170,
        ];

        // 1. Serialization is byte-stable for every variant.
        assert_eq!(
            bincode::serialize(&sample_put()).unwrap(),
            PUT,
            "Put (tag 0) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_update()).unwrap(),
            UPDATE,
            "Update (tag 1) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_get()).unwrap(),
            GET,
            "Get (tag 2) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_get_subscribe_pair()).unwrap(),
            GET_SUBSCRIBE_PAIR,
            "Get (tag 2) encoding changed (subscribe/blocking_subscribe pair)"
        );
        assert_eq!(
            bincode::serialize(&sample_subscribe()).unwrap(),
            SUBSCRIBE,
            "Subscribe (tag 3) encoding changed"
        );

        // 2. The four variant tags are exactly 0,1,2,3.
        assert_eq!(PUT[..4], 0u32.to_le_bytes());
        assert_eq!(UPDATE[..4], 1u32.to_le_bytes());
        assert_eq!(GET[..4], 2u32.to_le_bytes());
        assert_eq!(SUBSCRIBE[..4], 3u32.to_le_bytes());

        // 3. Each frozen byte stream still DECODES into its variant on this
        //    build.
        assert!(matches!(
            bincode::deserialize::<ContractRequest>(PUT).unwrap(),
            ContractRequest::Put { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<ContractRequest>(UPDATE).unwrap(),
            ContractRequest::Update { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<ContractRequest>(GET).unwrap(),
            ContractRequest::Get { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<ContractRequest>(SUBSCRIBE).unwrap(),
            ContractRequest::Subscribe { .. }
        ));
    }

    /// The flatbuffers union discriminants are fixed by
    /// `schemas/flatbuffers/client_request.fbs`'s declaration order for the
    /// `ContractRequestType` union (`Put, Update, Get, Subscribe` -> 1..4;
    /// `0` is the union's implicit `NONE`). Reordering that schema silently
    /// reassigns these the same way reordering the Rust enum reassigns the
    /// bincode tags above.
    #[test]
    fn fbs_discriminants_match_declaration_order() {
        assert_eq!(ContractRequestType::Put.0, 1);
        assert_eq!(ContractRequestType::Update.0, 2);
        assert_eq!(ContractRequestType::Get.0, 3);
        assert_eq!(ContractRequestType::Subscribe.0, 4);
    }
}

/// Wire-format pins for [`ClientRequest`].
///
/// Same rationale as `contract_request_wire_format` and
/// `delegate_request_wire_format` above. `ClientRequest` is the outermost
/// enum on the client<->node boundary: every one of its seven variants
/// (`DelegateOp`, `ContractOp`, `Disconnect`, `Authenticate`, `NodeQueries`,
/// `Close`, `StreamChunk`) is pinned to its FULL expected bincode byte
/// vector, which also transitively pins the nested `DelegateRequest` /
/// `ContractRequest` encodings for the two variants that wrap them. There
/// was no wire-format pin for `ClientRequest` before this module; this
/// establishes the frozen baseline from the current encoding.
///
/// Only 5 of the 7 variants exist on the flatbuffers wire path — the
/// `ClientRequestType` union in `client_request.fbs` declares
/// `ContractRequest, DelegateRequest, Disconnect, Authenticate, StreamChunk`
/// only; `NodeQueries` and `Close` are native-bincode-only (no fbs/browser
/// client constructs or sends them). Those five discriminants are pinned
/// too, for the same reason as `ContractRequestType` above.
#[cfg(test)]
mod client_request_wire_format {
    use super::{ClientRequest, ContractRequest, DelegateRequest, NodeQuery};
    use crate::code_hash::CodeHash;
    use crate::generated::client_request::ClientRequestType;
    use crate::prelude::{ContractInstanceId, DelegateKey};
    use bytes::Bytes;
    use std::borrow::Cow;

    // The seven sample values whose complete bincode encodings are frozen in
    // `wire_format_is_frozen`. Kept byte-for-byte identical to the throwaway
    // generator that produced the frozen vectors, so the freeze is
    // reproducible: construct the value, `bincode::serialize`, compare.
    fn sample_delegate_op() -> ClientRequest<'static> {
        ClientRequest::DelegateOp(DelegateRequest::UnregisterDelegate(DelegateKey::new(
            [0x11; 32],
            CodeHash::new([0x22; 32]),
        )))
    }

    fn sample_contract_op() -> ClientRequest<'static> {
        ClientRequest::ContractOp(ContractRequest::Subscribe {
            key: ContractInstanceId::new([0x33; 32]),
            summary: None,
        })
    }

    fn sample_disconnect() -> ClientRequest<'static> {
        ClientRequest::Disconnect {
            cause: Some(Cow::Borrowed("bye")),
        }
    }

    fn sample_authenticate() -> ClientRequest<'static> {
        ClientRequest::Authenticate {
            token: "tok".to_string(),
        }
    }

    fn sample_node_queries() -> ClientRequest<'static> {
        ClientRequest::NodeQueries(NodeQuery::ConnectedPeers)
    }

    fn sample_close() -> ClientRequest<'static> {
        ClientRequest::Close
    }

    fn sample_stream_chunk() -> ClientRequest<'static> {
        ClientRequest::StreamChunk {
            stream_id: 1,
            index: 2,
            total: 3,
            data: Bytes::from_static(&[9, 9, 9]),
        }
    }

    /// Complete-byte wire-format freeze for all seven variants.
    ///
    /// The test also DESERIALIZES each frozen vector, proving the byte
    /// stream still decodes into the expected variant on this build.
    #[test]
    fn wire_format_is_frozen() {
        const DELEGATE_OP: &[u8] = &[
            0, 0, 0, 0, 2, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
            17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 34, 34, 34, 34, 34, 34,
            34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34,
            34, 34, 34, 34,
        ];
        const CONTRACT_OP: &[u8] = &[
            1, 0, 0, 0, 3, 0, 0, 0, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51,
            51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 0,
        ];
        const DISCONNECT: &[u8] = &[2, 0, 0, 0, 1, 3, 0, 0, 0, 0, 0, 0, 0, 98, 121, 101];
        const AUTHENTICATE: &[u8] = &[3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 116, 111, 107];
        const NODE_QUERIES: &[u8] = &[4, 0, 0, 0, 0, 0, 0, 0];
        const CLOSE: &[u8] = &[5, 0, 0, 0];
        const STREAM_CHUNK: &[u8] = &[
            6, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 9, 9, 9,
        ];

        // 1. Serialization is byte-stable for every variant.
        assert_eq!(
            bincode::serialize(&sample_delegate_op()).unwrap(),
            DELEGATE_OP,
            "DelegateOp (tag 0) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_contract_op()).unwrap(),
            CONTRACT_OP,
            "ContractOp (tag 1) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_disconnect()).unwrap(),
            DISCONNECT,
            "Disconnect (tag 2) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_authenticate()).unwrap(),
            AUTHENTICATE,
            "Authenticate (tag 3) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_node_queries()).unwrap(),
            NODE_QUERIES,
            "NodeQueries (tag 4) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_close()).unwrap(),
            CLOSE,
            "Close (tag 5) encoding changed"
        );
        assert_eq!(
            bincode::serialize(&sample_stream_chunk()).unwrap(),
            STREAM_CHUNK,
            "StreamChunk (tag 6) encoding changed"
        );

        // 2. The seven variant tags are exactly 0,1,2,3,4,5,6.
        assert_eq!(DELEGATE_OP[..4], 0u32.to_le_bytes());
        assert_eq!(CONTRACT_OP[..4], 1u32.to_le_bytes());
        assert_eq!(DISCONNECT[..4], 2u32.to_le_bytes());
        assert_eq!(AUTHENTICATE[..4], 3u32.to_le_bytes());
        assert_eq!(NODE_QUERIES[..4], 4u32.to_le_bytes());
        assert_eq!(CLOSE[..4], 5u32.to_le_bytes());
        assert_eq!(STREAM_CHUNK[..4], 6u32.to_le_bytes());

        // 3. Each frozen byte stream still DECODES into its variant on this
        //    build.
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(DELEGATE_OP).unwrap(),
            ClientRequest::DelegateOp(_)
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(CONTRACT_OP).unwrap(),
            ClientRequest::ContractOp(_)
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(DISCONNECT).unwrap(),
            ClientRequest::Disconnect { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(AUTHENTICATE).unwrap(),
            ClientRequest::Authenticate { .. }
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(NODE_QUERIES).unwrap(),
            ClientRequest::NodeQueries(_)
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(CLOSE).unwrap(),
            ClientRequest::Close
        ));
        assert!(matches!(
            bincode::deserialize::<ClientRequest>(STREAM_CHUNK).unwrap(),
            ClientRequest::StreamChunk { .. }
        ));
    }

    /// The flatbuffers union discriminants are fixed by
    /// `schemas/flatbuffers/client_request.fbs`'s declaration order for the
    /// `ClientRequestType` union (`ContractRequest, DelegateRequest,
    /// Disconnect, Authenticate, StreamChunk` -> 1..5; `0` is the union's
    /// implicit `NONE`). Note the schema's `ContractRequest`/`DelegateRequest`
    /// ordering is the OPPOSITE of the Rust enum's `DelegateOp`/`ContractOp`
    /// ordering above -- the two encodings are independent wire contracts,
    /// each pinned to its own declaration order.
    #[test]
    fn fbs_discriminants_match_declaration_order() {
        assert_eq!(ClientRequestType::ContractRequest.0, 1);
        assert_eq!(ClientRequestType::DelegateRequest.0, 2);
        assert_eq!(ClientRequestType::Disconnect.0, 3);
        assert_eq!(ClientRequestType::Authenticate.0, 4);
        assert_eq!(ClientRequestType::StreamChunk.0, 5);
    }
}

/// Hardening pins for the flatbuffers decode boundary.
///
/// Every test here exists because a real decode site panicked, or silently
/// produced a wrong value, on input a client can actually send. The boundary
/// has exactly one entry point — [`ClientRequest::try_decode_fbs`] over
/// `root_as_client_request` — so these drive the real entry point rather than
/// an inner decoder wherever possible.
///
/// Two facts about the flatbuffers verifier make this whole class possible, and
/// both are load-bearing for every test below:
///
/// 1. A `(required)` vector is checked for PRESENCE, not LENGTH
///    (`Verifiable for Vector<T>` runs `verify_vector_range` and stops there).
/// 2. Every generated union verifier ends in `_ => Ok(())`, so an unknown
///    discriminant — including `NONE` — reaches the decoder's match.
#[cfg(test)]
mod fbs_decode_hardening {
    use super::{ClientRequest, ContractRequest};
    use crate::client_api::TryFromFbs;
    use crate::contract_interface::UpdateData;
    use crate::generated::client_request::{
        finish_client_request_buffer, ApplicationMessages, ApplicationMessagesArgs,
        ClientRequest as FbsClientRequest, ClientRequestArgs, ClientRequestType,
        ContractRequest as FbsContractRequest, ContractRequestArgs, ContractRequestType,
        DelegateCode as FbsDelegateCode, DelegateCodeArgs,
        DelegateContainer as FbsDelegateContainer, DelegateContainerArgs,
        DelegateKey as FbsDelegateKey, DelegateKeyArgs, DelegateRequest as FbsDelegateRequest,
        DelegateRequestArgs, DelegateRequestType, DelegateType, Get as FbsGet, GetArgs,
        InboundDelegateMsg as FbsInboundDelegateMsg, InboundDelegateMsgArgs,
        InboundDelegateMsgType, Put as FbsPut, PutArgs, RegisterDelegate, RegisterDelegateArgs,
        RelatedContract, RelatedContractArgs, RelatedContracts as FbsRelatedContracts,
        RelatedContractsArgs, Update as FbsUpdate, UpdateArgs, WasmDelegateV1, WasmDelegateV1Args,
    };
    use crate::generated::common::{
        ApplicationMessage as FbsApplicationMessage, ApplicationMessageArgs,
        ContractCode as FbsContractCode, ContractCodeArgs,
        ContractContainer as FbsContractContainer, ContractContainerArgs,
        ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs,
        ContractKey as FbsContractKey, ContractKeyArgs, ContractType, RelatedDeltaUpdate,
        RelatedDeltaUpdateArgs, RelatedStateAndDeltaUpdate, RelatedStateAndDeltaUpdateArgs,
        RelatedStateUpdate, RelatedStateUpdateArgs, StateUpdate, StateUpdateArgs,
        UpdateData as FbsUpdateData, UpdateDataArgs, UpdateDataType, WasmContractV1,
        WasmContractV1Args,
    };

    type Builder<'a> = flatbuffers::FlatBufferBuilder<'a>;

    /// The instance id the tests round-trip. Deliberately NOT all-ASCII: this is
    /// what a real 32-byte id looks like, and it is precisely what the old
    /// base58 decode choked on.
    const INSTANCE: [u8; 32] = [
        0x00, 0xff, 0x7a, 0x01, 0x30, 0x4f, 0x49, 0x6c, 0x2b, 0x2f, 0x5c, 0x7f, 0x80, 0xfe, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        0x20, 0x21,
    ];
    const CODE_HASH: [u8; 32] = [42u8; 32];

    fn instance_offset<'a>(
        b: &mut Builder<'a>,
        bytes: &[u8],
    ) -> flatbuffers::WIPOffset<FbsContractInstanceId<'a>> {
        let data = b.create_vector(bytes);
        FbsContractInstanceId::create(b, &ContractInstanceIdArgs { data: Some(data) })
    }

    fn key_offset<'a>(
        b: &mut Builder<'a>,
        instance: &[u8],
        code: &[u8],
    ) -> flatbuffers::WIPOffset<FbsContractKey<'a>> {
        let instance = instance_offset(b, instance);
        let code = b.create_vector(code);
        FbsContractKey::create(
            b,
            &ContractKeyArgs {
                instance: Some(instance),
                code: Some(code),
            },
        )
    }

    fn delegate_key_offset<'a>(
        b: &mut Builder<'a>,
        key: &[u8],
        code_hash: &[u8],
    ) -> flatbuffers::WIPOffset<FbsDelegateKey<'a>> {
        let key = b.create_vector(key);
        let code_hash = b.create_vector(code_hash);
        FbsDelegateKey::create(
            b,
            &DelegateKeyArgs {
                key: Some(key),
                code_hash: Some(code_hash),
            },
        )
    }

    /// Finish a `ContractRequest`-carrying `ClientRequest` and return its bytes.
    fn finish_contract(
        b: &mut Builder<'_>,
        ty: ContractRequestType,
        req: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) -> Vec<u8> {
        let contract = FbsContractRequest::create(
            b,
            &ContractRequestArgs {
                contract_request_type: ty,
                contract_request: Some(req),
            },
        );
        let client = FbsClientRequest::create(
            b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::ContractRequest,
                client_request: Some(contract.as_union_value()),
            },
        );
        finish_client_request_buffer(b, client);
        b.finished_data().to_vec()
    }

    fn finish_delegate(
        b: &mut Builder<'_>,
        ty: DelegateRequestType,
        req: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) -> Vec<u8> {
        let delegate = FbsDelegateRequest::create(
            b,
            &DelegateRequestArgs {
                delegate_request_type: ty,
                delegate_request: Some(req),
            },
        );
        let client = FbsClientRequest::create(
            b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType::DelegateRequest,
                client_request: Some(delegate.as_union_value()),
            },
        );
        finish_client_request_buffer(b, client);
        b.finished_data().to_vec()
    }

    // ---------------------------------------------------------------------
    // Per-union builders. Each produces a well-formed request whose ONE named
    // union discriminant is `d`, so a test can sweep `d` while everything else
    // stays valid.
    // ---------------------------------------------------------------------

    fn client_request_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let get = FbsGet::create(
            &mut b,
            &GetArgs {
                key: Some(key),
                fetch_contract: false,
                subscribe: false,
                blocking_subscribe: false,
            },
        );
        let contract = FbsContractRequest::create(
            &mut b,
            &ContractRequestArgs {
                contract_request_type: ContractRequestType::Get,
                contract_request: Some(get.as_union_value()),
            },
        );
        let client = FbsClientRequest::create(
            &mut b,
            &ClientRequestArgs {
                client_request_type: ClientRequestType(d),
                client_request: Some(contract.as_union_value()),
            },
        );
        finish_client_request_buffer(&mut b, client);
        b.finished_data().to_vec()
    }

    fn contract_request_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let get = FbsGet::create(
            &mut b,
            &GetArgs {
                key: Some(key),
                fetch_contract: false,
                subscribe: false,
                blocking_subscribe: false,
            },
        );
        finish_contract(&mut b, ContractRequestType(d), get.as_union_value())
    }

    fn delegate_request_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let dk = delegate_key_offset(&mut b, &[7u8; 32], &CODE_HASH);
        let params = b.create_vector(&[1u8, 2, 3]);
        let payload = b.create_vector(&[9u8; 4]);
        let context = b.create_vector(&[0u8; 2]);
        let app = FbsApplicationMessage::create(
            &mut b,
            &ApplicationMessageArgs {
                payload: Some(payload),
                context: Some(context),
                processed: false,
            },
        );
        let inbound_msg = FbsInboundDelegateMsg::create(
            &mut b,
            &InboundDelegateMsgArgs {
                inbound_type: InboundDelegateMsgType::common_ApplicationMessage,
                inbound: Some(app.as_union_value()),
            },
        );
        let inbound = b.create_vector(&[inbound_msg]);
        let msgs = ApplicationMessages::create(
            &mut b,
            &ApplicationMessagesArgs {
                key: Some(dk),
                params: Some(params),
                inbound: Some(inbound),
            },
        );
        finish_delegate(&mut b, DelegateRequestType(d), msgs.as_union_value())
    }

    /// A PUT whose `common.ContractContainer` union discriminant is `d`.
    fn contract_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let code_data = b.create_vector(&[0u8; 8]);
        let code_hash = b.create_vector(&CODE_HASH);
        let code = FbsContractCode::create(
            &mut b,
            &ContractCodeArgs {
                data: Some(code_data),
                code_hash: Some(code_hash),
            },
        );
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let params = b.create_vector(&[1u8, 2]);
        let wasm = WasmContractV1::create(
            &mut b,
            &WasmContractV1Args {
                data: Some(code),
                parameters: Some(params),
                key: Some(key),
            },
        );
        let container = FbsContractContainer::create(
            &mut b,
            &ContractContainerArgs {
                contract_type: ContractType(d),
                contract: Some(wasm.as_union_value()),
            },
        );
        let state = b.create_vector(&[3u8; 4]);
        let empty: Vec<flatbuffers::WIPOffset<RelatedContract>> = vec![];
        let contracts = b.create_vector(&empty);
        let related = FbsRelatedContracts::create(
            &mut b,
            &RelatedContractsArgs {
                contracts: Some(contracts),
            },
        );
        let put = FbsPut::create(
            &mut b,
            &PutArgs {
                container: Some(container),
                wrapped_state: Some(state),
                related_contracts: Some(related),
                subscribe: false,
                blocking_subscribe: false,
            },
        );
        finish_contract(&mut b, ContractRequestType::Put, put.as_union_value())
    }

    /// A RegisterDelegate whose `DelegateContainer` union discriminant is `d`.
    fn delegate_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let code_data = b.create_vector(&[0u8; 8]);
        let code_hash = b.create_vector(&CODE_HASH);
        let code = FbsDelegateCode::create(
            &mut b,
            &DelegateCodeArgs {
                data: Some(code_data),
                code_hash: Some(code_hash),
            },
        );
        let dk = delegate_key_offset(&mut b, &[7u8; 32], &CODE_HASH);
        let params = b.create_vector(&[1u8, 2]);
        let wasm = WasmDelegateV1::create(
            &mut b,
            &WasmDelegateV1Args {
                parameters: Some(params),
                data: Some(code),
                key: Some(dk),
            },
        );
        let container = FbsDelegateContainer::create(
            &mut b,
            &DelegateContainerArgs {
                delegate_type: DelegateType(d),
                delegate: Some(wasm.as_union_value()),
            },
        );
        let cipher = b.create_vector(&[1u8; 32]);
        let nonce = b.create_vector(&[2u8; 24]);
        let register = RegisterDelegate::create(
            &mut b,
            &RegisterDelegateArgs {
                delegate: Some(container),
                cipher: Some(cipher),
                nonce: Some(nonce),
            },
        );
        finish_delegate(
            &mut b,
            DelegateRequestType::RegisterDelegate,
            register.as_union_value(),
        )
    }

    /// An UPDATE whose `common.UpdateData` union discriminant is `d`.
    fn update_data_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let state = b.create_vector(&[5u8; 4]);
        let state_update = StateUpdate::create(&mut b, &StateUpdateArgs { state: Some(state) });
        let data = FbsUpdateData::create(
            &mut b,
            &UpdateDataArgs {
                update_data_type: UpdateDataType(d),
                update_data: Some(state_update.as_union_value()),
            },
        );
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let update = FbsUpdate::create(
            &mut b,
            &UpdateArgs {
                key: Some(key),
                data: Some(data),
            },
        );
        finish_contract(&mut b, ContractRequestType::Update, update.as_union_value())
    }

    /// An ApplicationMessages whose `InboundDelegateMsg` union discriminant is `d`.
    fn inbound_delegate_msg_type(d: u8, force_defaults: bool) -> Vec<u8> {
        let mut b = Builder::new();
        b.force_defaults(force_defaults);
        let payload = b.create_vector(&[9u8; 4]);
        let context = b.create_vector(&[0u8; 2]);
        let app = FbsApplicationMessage::create(
            &mut b,
            &ApplicationMessageArgs {
                payload: Some(payload),
                context: Some(context),
                processed: false,
            },
        );
        let inbound_msg = FbsInboundDelegateMsg::create(
            &mut b,
            &InboundDelegateMsgArgs {
                inbound_type: InboundDelegateMsgType(d),
                inbound: Some(app.as_union_value()),
            },
        );
        let inbound = b.create_vector(&[inbound_msg]);
        let dk = delegate_key_offset(&mut b, &[7u8; 32], &CODE_HASH);
        let params = b.create_vector(&[1u8, 2, 3]);
        let msgs = ApplicationMessages::create(
            &mut b,
            &ApplicationMessagesArgs {
                key: Some(dk),
                params: Some(params),
                inbound: Some(inbound),
            },
        );
        finish_delegate(
            &mut b,
            DelegateRequestType::ApplicationMessages,
            msgs.as_union_value(),
        )
    }

    /// `(union name, builder, the union's real discriminant)`.
    type UnionCase = (&'static str, fn(u8, bool) -> Vec<u8>, u8);

    /// **No union discriminant, for any of the seven unions on the decode path,
    /// may panic the decoder.**
    ///
    /// One sweep covering every union at once. Four of the seven were
    /// `unreachable!()` before this change, and all four are reachable: the
    /// generated verifiers end in `_ => Ok(())`, so an unrecognized discriminant
    /// passes verification and lands in the decoder's match.
    ///
    /// Scope, stated honestly: this covers a new *variant* on an existing union
    /// for free, but NOT a new *union* — `cases` and its builders are written by
    /// hand. `union_matches_never_use_unreachable` is the guard for that case.
    ///
    /// Four properties are pinned per union, and the third is the one that keeps
    /// the other three honest:
    ///
    /// - The 0..=255 sweep RETURNS. A panic anywhere inside fails the test.
    /// - An out-of-range discriminant is a clean error.
    /// - That error came from the DECODER, not the verifier. Without this the
    ///   sweep silently degrades to zero decoder coverage the moment a schema or
    ///   builder change makes verification fail earlier — which is exactly what
    ///   had happened to the NONE case below.
    /// - The real discriminant still decodes, so the guard did not break the
    ///   happy path.
    #[test]
    fn no_union_discriminant_panics_the_decoder() {
        let cases: [UnionCase; 7] = [
            ("ClientRequestType", client_request_type, 1),
            ("ContractRequestType", contract_request_type, 3),
            ("DelegateRequestType", delegate_request_type, 1),
            ("ContractType", contract_type, 1),
            ("DelegateType", delegate_type, 1),
            ("UpdateDataType", update_data_type, 1),
            (
                "InboundDelegateMsgType",
                inbound_delegate_msg_type,
                InboundDelegateMsgType::common_ApplicationMessage.0,
            ),
        ];

        for (union, build, valid) in cases {
            for d in 0..=u8::MAX {
                // Must return, never panic. A mismatched-but-known discriminant
                // may be rejected by the verifier rather than the decoder;
                // either is a clean error and both are acceptable here.
                let bytes = build(d, false);
                let _ = ClientRequest::try_decode_fbs(&bytes);
            }

            // An out-of-range discriminant must be rejected BY THE DECODER.
            // Asserting only `is_err()` would also pass on a verifier
            // rejection, which reaches none of the code this change touches.
            let out_of_range = build(200, false);
            let err = ClientRequest::try_decode_fbs(&out_of_range)
                .expect_err("{union}: an out-of-range discriminant must be a clean error");
            assert_eq!(
                err.to_string(),
                format!(
                    "Failed decoding message from client request: unknown {union} \
                     discriminant: 200"
                ),
                "{union}: the error must come from the decoder's union arm, not \
                 from the verifier — otherwise this sweep pins nothing"
            );

            // `NONE` needs `force_defaults`: both builders elide a field equal
            // to its default, so an ordinary `NONE` is written as ABSENT and
            // `visit_union` rejects the present-value/absent-type pair before
            // any decoder runs. Forcing defaults writes the zero explicitly,
            // which is the shape a non-flatc encoder can put on the wire and
            // the only one that actually reaches the match arm.
            let none = build(0, true);
            let err = ClientRequest::try_decode_fbs(&none)
                .expect_err("{union}: a NONE discriminant must be a clean error");
            assert_eq!(
                err.to_string(),
                format!(
                    "Failed decoding message from client request: unknown {union} \
                     discriminant: 0"
                ),
                "{union}: an explicit NONE must reach the decoder's union arm"
            );

            let good = build(valid, false);
            assert!(
                ClientRequest::try_decode_fbs(&good).is_ok(),
                "{union}: the real discriminant must still decode; the guard \
                 must not break the happy path"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Per-site pins. Each of these fails if its own decode site is reverted,
    // so a partial revert cannot leave the suite green — the failure mode #85
    // shipped with, where the fix reached three call sites and only one was
    // pinned.
    // ---------------------------------------------------------------------

    /// Build an UPDATE carrying one of the three `related_*` update variants,
    /// with `id` as the raw `related_to` bytes.
    fn update_with_related(variant: UpdateDataType, id: &[u8]) -> Vec<u8> {
        let mut b = Builder::new();
        let related_to = instance_offset(&mut b, id);
        let payload = b.create_vector(&[5u8; 4]);
        let data_offset = match variant {
            UpdateDataType::RelatedStateUpdate => RelatedStateUpdate::create(
                &mut b,
                &RelatedStateUpdateArgs {
                    related_to: Some(related_to),
                    state: Some(payload),
                },
            )
            .as_union_value(),
            UpdateDataType::RelatedDeltaUpdate => RelatedDeltaUpdate::create(
                &mut b,
                &RelatedDeltaUpdateArgs {
                    related_to: Some(related_to),
                    delta: Some(payload),
                },
            )
            .as_union_value(),
            UpdateDataType::RelatedStateAndDeltaUpdate => {
                let delta = b.create_vector(&[6u8; 4]);
                RelatedStateAndDeltaUpdate::create(
                    &mut b,
                    &RelatedStateAndDeltaUpdateArgs {
                        related_to: Some(related_to),
                        state: Some(payload),
                        delta: Some(delta),
                    },
                )
                .as_union_value()
            }
            other => panic!("not a related update variant: {}", other.0),
        };
        let data = FbsUpdateData::create(
            &mut b,
            &UpdateDataArgs {
                update_data_type: variant,
                update_data: Some(data_offset),
            },
        );
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let update = FbsUpdate::create(
            &mut b,
            &UpdateArgs {
                key: Some(key),
                data: Some(data),
            },
        );
        finish_contract(&mut b, ContractRequestType::Update, update.as_union_value())
    }

    fn decoded_related_to(bytes: &[u8]) -> [u8; 32] {
        let req = ClientRequest::try_decode_fbs(bytes)
            .expect("a well-formed related update must decode, not panic");
        let ClientRequest::ContractOp(ContractRequest::Update { data, .. }) = req else {
            panic!("expected an UPDATE, got {req:?}");
        };
        match data {
            UpdateData::RelatedState { related_to, .. }
            | UpdateData::RelatedDelta { related_to, .. }
            | UpdateData::RelatedStateAndDelta { related_to, .. } => *related_to,
            other => panic!("expected a related update, got {other:?}"),
        }
    }

    /// `related_to` is 32 RAW bytes and must round-trip byte-for-byte.
    ///
    /// This decode used to be `ContractInstanceId::from_bytes(..).unwrap()` —
    /// a base58 *string* decoder pointed at bytes that are already the final
    /// id. It did not merely mishandle malformed input: a random 32-byte id
    /// essentially never consists solely of base58 characters, so EVERY
    /// well-formed related update panicked the connection task. `INSTANCE`
    /// contains `0x00`, `0xff` and the base58-excluded `0`/`O`/`I`/`l`
    /// characters precisely so a revert cannot pass by luck.
    #[test]
    fn related_state_update_round_trips_the_raw_instance_id() {
        let bytes = update_with_related(UpdateDataType::RelatedStateUpdate, &INSTANCE);
        assert_eq!(decoded_related_to(&bytes), INSTANCE);
    }

    #[test]
    fn related_delta_update_round_trips_the_raw_instance_id() {
        let bytes = update_with_related(UpdateDataType::RelatedDeltaUpdate, &INSTANCE);
        assert_eq!(decoded_related_to(&bytes), INSTANCE);
    }

    #[test]
    fn related_state_and_delta_update_round_trips_the_raw_instance_id() {
        let bytes = update_with_related(UpdateDataType::RelatedStateAndDeltaUpdate, &INSTANCE);
        assert_eq!(decoded_related_to(&bytes), INSTANCE);
    }

    /// A wrong-length `related_to` is a clean error naming the field, not a
    /// panic and not a silently truncated id.
    ///
    /// Parameterized over all three variants: with one case, copy-pasting the
    /// wrong variant's field name into the other two arms would fail nothing.
    #[test]
    fn related_to_wrong_length_is_rejected() {
        for (variant, field) in [
            (
                UpdateDataType::RelatedStateUpdate,
                "RelatedStateUpdate.related_to.data",
            ),
            (
                UpdateDataType::RelatedDeltaUpdate,
                "RelatedDeltaUpdate.related_to.data",
            ),
            (
                UpdateDataType::RelatedStateAndDeltaUpdate,
                "RelatedStateAndDeltaUpdate.related_to.data",
            ),
        ] {
            let bytes = update_with_related(variant, &[1u8; 8]);
            let err = ClientRequest::try_decode_fbs(&bytes)
                .expect_err("an 8-byte related_to must be rejected");
            let msg = err.to_string();
            assert!(
                msg.contains(field) && msg.contains("got 8 bytes"),
                "the error must name {field} and the observed length, got: {msg}"
            );
        }
    }

    fn put_with_related_contract(id: &[u8]) -> Vec<u8> {
        let mut b = Builder::new();
        let code_data = b.create_vector(&[0u8; 8]);
        let code_hash = b.create_vector(&CODE_HASH);
        let code = FbsContractCode::create(
            &mut b,
            &ContractCodeArgs {
                data: Some(code_data),
                code_hash: Some(code_hash),
            },
        );
        let key = key_offset(&mut b, &INSTANCE, &CODE_HASH);
        let params = b.create_vector(&[1u8, 2]);
        let wasm = WasmContractV1::create(
            &mut b,
            &WasmContractV1Args {
                data: Some(code),
                parameters: Some(params),
                key: Some(key),
            },
        );
        let container = FbsContractContainer::create(
            &mut b,
            &ContractContainerArgs {
                contract_type: ContractType::WasmContractV1,
                contract: Some(wasm.as_union_value()),
            },
        );
        let related_id = instance_offset(&mut b, id);
        let related_state = b.create_vector(&[8u8; 3]);
        let related_contract = RelatedContract::create(
            &mut b,
            &RelatedContractArgs {
                instance_id: Some(related_id),
                state: Some(related_state),
            },
        );
        let contracts = b.create_vector(&[related_contract]);
        let related = FbsRelatedContracts::create(
            &mut b,
            &RelatedContractsArgs {
                contracts: Some(contracts),
            },
        );
        let state = b.create_vector(&[3u8; 4]);
        let put = FbsPut::create(
            &mut b,
            &PutArgs {
                container: Some(container),
                wrapped_state: Some(state),
                related_contracts: Some(related),
                subscribe: false,
                blocking_subscribe: false,
            },
        );
        finish_contract(&mut b, ContractRequestType::Put, put.as_union_value())
    }

    /// The same base58 bug lived on the PUT path, in `RelatedContracts`. It went
    /// unnoticed because the loop body only runs when the vector is NON-empty,
    /// and the TypeScript suite's PUT fixture passes `new RelatedContractsT([])`.
    #[test]
    fn put_related_contract_round_trips_the_raw_instance_id() {
        let bytes = put_with_related_contract(&INSTANCE);
        let req = ClientRequest::try_decode_fbs(&bytes)
            .expect("a PUT carrying a related contract must decode, not panic");
        let ClientRequest::ContractOp(ContractRequest::Put {
            related_contracts, ..
        }) = req
        else {
            panic!("expected a PUT, got {req:?}");
        };
        let ids: Vec<[u8; 32]> = related_contracts
            .into_owned()
            .states()
            .map(|(id, _)| **id)
            .collect();
        assert_eq!(
            ids,
            vec![INSTANCE],
            "the related contract id must round-trip"
        );
    }

    #[test]
    fn put_related_contract_wrong_length_id_is_rejected() {
        let bytes = put_with_related_contract(&[1u8; 8]);
        let err = ClientRequest::try_decode_fbs(&bytes)
            .expect_err("an 8-byte related contract id must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("RelatedContract.instance_id") && msg.contains("got 8 bytes"),
            "got: {msg}"
        );
    }

    fn unregister_delegate(key_len: usize) -> Vec<u8> {
        use crate::generated::client_request::{UnregisterDelegate, UnregisterDelegateArgs};
        let mut b = Builder::new();
        let dk = delegate_key_offset(&mut b, &vec![7u8; key_len], &CODE_HASH);
        let unregister =
            UnregisterDelegate::create(&mut b, &UnregisterDelegateArgs { key: Some(dk) });
        finish_delegate(
            &mut b,
            DelegateRequestType::UnregisterDelegate,
            unregister.as_union_value(),
        )
    }

    /// `DelegateKey.key` is `(required)` but length-unchecked by the verifier,
    /// and the decoder used to `copy_from_slice` it into a `[0; 32]` — which
    /// panics on a mismatch. This is the normal delegate path, and the
    /// TypeScript SDK exports `DelegateKey` as the raw generated type with no
    /// length validation (unlike `ContractKey`, which throws a `TypeError`).
    #[test]
    fn delegate_key_wrong_length_is_rejected_not_panicking() {
        let short = unregister_delegate(8);
        let err = ClientRequest::try_decode_fbs(&short)
            .expect_err("an 8-byte delegate key must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("DelegateKey.key") && msg.contains("got 8 bytes"),
            "got: {msg}"
        );

        let long = unregister_delegate(64);
        let err = ClientRequest::try_decode_fbs(&long)
            .expect_err("a 64-byte delegate key must be rejected");
        assert!(err.to_string().contains("got 64 bytes"), "got: {err}");

        let good = unregister_delegate(32);
        assert!(
            ClientRequest::try_decode_fbs(&good).is_ok(),
            "a 32-byte delegate key must still decode"
        );
    }

    fn register_delegate(cipher_len: usize, nonce_len: usize) -> Vec<u8> {
        let mut b = Builder::new();
        let code_data = b.create_vector(&[0u8; 8]);
        let code_hash = b.create_vector(&CODE_HASH);
        let code = FbsDelegateCode::create(
            &mut b,
            &DelegateCodeArgs {
                data: Some(code_data),
                code_hash: Some(code_hash),
            },
        );
        let dk = delegate_key_offset(&mut b, &[7u8; 32], &CODE_HASH);
        let params = b.create_vector(&[1u8, 2]);
        let wasm = WasmDelegateV1::create(
            &mut b,
            &WasmDelegateV1Args {
                parameters: Some(params),
                data: Some(code),
                key: Some(dk),
            },
        );
        let container = FbsDelegateContainer::create(
            &mut b,
            &DelegateContainerArgs {
                delegate_type: DelegateType::WasmDelegateV1,
                delegate: Some(wasm.as_union_value()),
            },
        );
        let cipher = b.create_vector(&vec![1u8; cipher_len]);
        let nonce = b.create_vector(&vec![2u8; nonce_len]);
        let register = RegisterDelegate::create(
            &mut b,
            &RegisterDelegateArgs {
                delegate: Some(container),
                cipher: Some(cipher),
                nonce: Some(nonce),
            },
        );
        finish_delegate(
            &mut b,
            DelegateRequestType::RegisterDelegate,
            register.as_union_value(),
        )
    }

    /// `cipher` (32) and `nonce` (24) are the same shape: `(required)`, so the
    /// verifier guarantees presence and nothing about length, and the decoder
    /// used to `try_from(..).unwrap()` them.
    #[test]
    fn register_delegate_wrong_length_cipher_or_nonce_is_rejected() {
        let err = ClientRequest::try_decode_fbs(&register_delegate(16, 24))
            .expect_err("a 16-byte cipher must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("RegisterDelegate.cipher") && msg.contains("got 16 bytes"),
            "got: {msg}"
        );

        let err = ClientRequest::try_decode_fbs(&register_delegate(32, 8))
            .expect_err("an 8-byte nonce must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("RegisterDelegate.nonce") && msg.contains("got 8 bytes"),
            "got: {msg}"
        );

        assert!(
            ClientRequest::try_decode_fbs(&register_delegate(32, 24)).is_ok(),
            "correct cipher/nonce lengths must still decode"
        );
    }

    /// The ENCODE half of the same bug: `HostResponse`'s three related-update
    /// variants wrote `related_to.encode()` — base58 TEXT — into
    /// `common.ContractInstanceId.data`, a field every other producer and every
    /// consumer treats as 32 raw bytes.
    ///
    /// It survived because Rust only encodes host responses and only TypeScript
    /// decodes them, so no Rust round-trip test ever crossed it, and the
    /// TypeScript SDK has no test that reads `relatedTo` back out of a
    /// notification. Pinned here by decoding the encoder's own output, which is
    /// the cheapest thing that would have caught it.
    #[test]
    fn host_response_encodes_related_to_as_raw_bytes() {
        use crate::client_api::{ContractResponse, HostResponse};
        use crate::contract_interface::{ContractInstanceId, ContractKey, State};
        use crate::generated::host_response::{root_as_host_response, ContractResponseType};

        let related = ContractInstanceId::new(INSTANCE);
        let key = ContractKey::from_params_and_code(
            crate::parameters::Parameters::from(vec![1u8, 2]),
            crate::contract_interface::ContractCode::from(vec![0u8; 8]),
        );
        let response = HostResponse::ContractResponse(ContractResponse::UpdateNotification {
            key,
            update: UpdateData::RelatedState {
                related_to: related,
                state: State::from(vec![9u8; 4]),
            },
        });

        let bytes = response.into_fbs_bytes().expect("encoding must succeed");
        let host = root_as_host_response(&bytes).expect("the encoder must emit a valid buffer");
        let contract = host
            .response_as_contract_response()
            .expect("a ContractResponse");
        assert_eq!(
            contract.contract_response_type(),
            ContractResponseType::UpdateNotification
        );
        let notification = contract
            .contract_response_as_update_notification()
            .expect("an UpdateNotification");
        let related_update = notification
            .update()
            .update_data_as_related_state_update()
            .expect("a RelatedStateUpdate");

        assert_eq!(
            related_update.related_to().data().bytes(),
            &INSTANCE,
            "related_to must be the 32 RAW id bytes. Encoding it as base58 text \
             puts ~44 ASCII bytes in a field the TypeScript SDK reads as a raw \
             Uint8Array, and that our own decoder now rejects."
        );
    }

    /// `SecretsId::try_decode_fbs` has no production caller today, so it is
    /// pinned directly rather than through a request. Fixing it now means the
    /// first client to reach it does not find a panic waiting.
    #[test]
    fn secrets_id_wrong_length_hash_is_rejected_not_panicking() {
        use crate::delegate_interface::SecretsId;
        use crate::generated::common::{SecretsId as FbsSecretsId, SecretsIdArgs};

        let build = |hash_len: usize| {
            let mut b = Builder::new();
            let key = b.create_vector(&[1u8, 2, 3]);
            let hash = b.create_vector(&vec![4u8; hash_len]);
            let id = FbsSecretsId::create(
                &mut b,
                &SecretsIdArgs {
                    key: Some(key),
                    hash: Some(hash),
                },
            );
            b.finish_minimal(id);
            b.finished_data().to_vec()
        };

        let bytes = build(8);
        let fbs = flatbuffers::root::<FbsSecretsId>(&bytes)
            .expect("the verifier accepts a short required vector");
        let err = SecretsId::try_decode_fbs(&fbs).expect_err("an 8-byte hash must be rejected");
        assert!(
            err.to_string().contains("SecretsId.hash") && err.to_string().contains("got 8 bytes"),
            "got: {err}"
        );

        let bytes = build(32);
        let fbs = flatbuffers::root::<FbsSecretsId>(&bytes).expect("well-formed");
        assert!(
            SecretsId::try_decode_fbs(&fbs).is_ok(),
            "a 32-byte hash must still decode"
        );
    }
}

/// Executable evidence for what happens on the bincode wire when a **field** is
/// appended to a struct.
///
/// This is a different question from appending a variant to an enum, and the
/// answers are not merely different — they break in the **opposite direction**.
/// Getting that backwards is easy and expensive, so the behaviour is pinned
/// here rather than reasoned about in a review comment.
///
/// Summary of what these tests establish, for `bincode::serialize` /
/// `bincode::deserialize` as this crate uses them:
///
/// | change | old sender to new receiver | new sender to old receiver |
/// |---|---|---|
/// | append an **enum variant** | fine (old tags unchanged) | hard error, unknown tag |
/// | append a **struct field** | **hard error**, unexpected end of input | silently ignored *if the struct is terminal*, **silent corruption** if it is not |
///
/// So a struct field is the more dangerous of the two. It has no tag, so there
/// is nothing for a decoder to skip; bincode is positional and not
/// self-describing. `#[serde(default)]` does not help — see
/// `serde_default_does_not_rescue_a_missing_bincode_field`.
///
/// The practical rule that follows: **prefer a new enum variant over a new
/// field on an existing wire struct.** A variant is only seen by a peer that
/// asked for it; a field changes what every existing peer decodes.
#[cfg(test)]
mod struct_field_wire_compat {
    use super::{HostResponse, NodeDiagnosticsResponse, QueryResponse, WrappedState};
    use serde::{Deserialize, Serialize};

    /// A wire struct before a field was appended.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OldShape {
        first: u32,
        second: String,
    }

    /// The same struct after appending `added`, the way a new node would emit it.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct NewShape {
        first: u32,
        second: String,
        added: Vec<u8>,
    }

    /// The same again, but with `#[serde(default)]` on the new field — the
    /// annotation people reach for expecting it to make the change compatible.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct NewShapeWithDefault {
        first: u32,
        second: String,
        #[serde(default)]
        added: Vec<u8>,
    }

    fn old_value() -> OldShape {
        OldShape {
            first: 7,
            second: "diagnostics".to_string(),
        }
    }

    fn new_value() -> NewShape {
        NewShape {
            first: 7,
            second: "diagnostics".to_string(),
            added: vec![0xDE, 0xAD],
        }
    }

    /// **New sender to old receiver, struct terminal: silently succeeds, and
    /// the new field is dropped.**
    ///
    /// `bincode::deserialize` is configured `allow_trailing_bytes()`
    /// (bincode-1.3.3 `src/lib.rs`), so the appended field's bytes are simply
    /// left unread. No error, no warning — the old peer just never learns the
    /// field exists.
    ///
    /// This is the benign case, and it is benign *only* because nothing follows
    /// the struct in the encoding. See
    /// `an_appended_field_corrupts_whatever_follows_it`.
    #[test]
    fn a_new_field_is_silently_ignored_by_an_old_receiver() {
        let bytes = bincode::serialize(&new_value()).expect("new value must serialize");
        let decoded: OldShape =
            bincode::deserialize(&bytes).expect("trailing bytes are allowed, so this succeeds");
        assert_eq!(decoded, old_value(), "the shared prefix decodes unchanged");
    }

    /// **Old sender to new receiver: hard decode error.**
    ///
    /// This is the direction that bites, and it is the reverse of the enum
    /// case. A new client reading an old node's response runs off the end of
    /// the input looking for a field the old node never wrote, and the whole
    /// message fails — not just the new field.
    ///
    /// Concretely: appending a field to a response struct means a freshly-built
    /// client cannot decode that response from **any** node not yet upgraded.
    /// During a staged fleet rollout that is most of the fleet, and the tool
    /// you would use to watch the rollout is the one that breaks.
    #[test]
    fn an_old_payload_fails_to_decode_once_a_field_is_appended() {
        let bytes = bincode::serialize(&old_value()).expect("old value must serialize");
        let decoded = bincode::deserialize::<NewShape>(&bytes);
        assert!(
            decoded.is_err(),
            "an old payload must NOT decode into a struct with an appended field; \
             if this ever passes, the compatibility table on this module is wrong"
        );
    }

    /// `#[serde(default)]` does **not** rescue the case above.
    ///
    /// It is a self-describing-format feature: it fills a field whose *name*
    /// was absent from the input. bincode carries no names and no field count,
    /// so there is no "absent" to detect — the decoder just reads past the end
    /// of the buffer and fails. `#[serde(default)]` on a bincode struct field
    /// is therefore JSON-only protection, and reading it as wire compatibility
    /// is a mistake worth naming explicitly.
    ///
    /// (`ContractState::size_bytes` in this file carries exactly this
    /// annotation. It protects the `serde_json` report path, not the bincode
    /// client path.)
    #[test]
    fn serde_default_does_not_rescue_a_missing_bincode_field() {
        let bytes = bincode::serialize(&old_value()).expect("old value must serialize");
        assert!(
            bincode::deserialize::<NewShapeWithDefault>(&bytes).is_err(),
            "#[serde(default)] must not be mistaken for bincode wire compatibility"
        );

        // The same annotation genuinely does work for JSON, which is why it is
        // easy to believe it works everywhere.
        let json = serde_json::to_string(&old_value()).expect("old value must serialize to JSON");
        let from_json: NewShapeWithDefault =
            serde_json::from_str(&json).expect("serde(default) fills the missing field in JSON");
        assert!(from_json.added.is_empty());
    }

    /// **The dangerous case: an appended field that is not terminal corrupts
    /// whatever follows it, silently.**
    ///
    /// If the struct has siblings after it in the enclosing encoding, the extra
    /// bytes are not trailing — they shift every subsequent field. An old
    /// receiver then reads the new field's bytes *as* the next field and gets a
    /// plausible, wrong value with no error at all.
    ///
    /// This is why "adding a field is fine, we checked" is not a conclusion you
    /// can carry from one struct to another: whether it is safe depends on
    /// where the struct sits in the message, not on the struct.
    #[test]
    fn an_appended_field_corrupts_whatever_follows_it() {
        #[allow(dead_code)] // `payload` exists to occupy wire space, not to be read
        #[derive(Serialize, Deserialize, Debug)]
        struct OldEnvelope {
            payload: OldShape,
            trailer: u32,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct NewEnvelope {
            payload: NewShape,
            trailer: u32,
        }

        let bytes = bincode::serialize(&NewEnvelope {
            payload: new_value(),
            trailer: 0xABCD_EF01,
        })
        .expect("new envelope must serialize");

        match bincode::deserialize::<OldEnvelope>(&bytes) {
            Ok(decoded) => assert_ne!(
                decoded.trailer, 0xABCD_EF01,
                "if this ever holds, bincode grew field framing and this whole module \
                 needs revisiting"
            ),
            Err(_) => {
                // Also an acceptable outcome, and the better one: the shifted
                // bytes happened not to form a decodable value. The point of
                // the test is that the field is NOT skipped cleanly, and both
                // arms show that.
            }
        }
    }

    /// `NodeDiagnosticsResponse` is **terminal** in its enclosing message, and
    /// this pins that property.
    ///
    /// It is the only reason appending a field to it is survivable for old
    /// clients at all (`a_new_field_is_silently_ignored_by_an_old_receiver`).
    /// The property is invisible at the definition site — nothing next to
    /// `NodeDiagnosticsResponse` says "must stay last" — so if a field is ever
    /// added *after* the payload in `QueryResponse::NodeDiagnostics` or
    /// `HostResponse::QueryResponse`, this fails and says why.
    ///
    /// Note what this does NOT license: appending to `NodeDiagnosticsResponse`
    /// still breaks a new client talking to an old node, per
    /// `an_old_payload_fails_to_decode_once_a_field_is_appended`. Terminality
    /// buys one direction, not both.
    #[test]
    fn node_diagnostics_response_is_terminal_in_its_message() {
        let response = NodeDiagnosticsResponse {
            node_info: None,
            network_info: None,
            subscriptions: vec![],
            contract_states: Default::default(),
            system_metrics: None,
            connected_peers_detailed: vec![],
        };

        let inner = bincode::serialize(&response).expect("response must serialize");
        // The default type parameter, i.e. the `HostResponse` that is actually
        // on the wire. Pinning terminality against some other instantiation
        // would be pinning a type nobody sends.
        let whole = bincode::serialize(&HostResponse::<WrappedState>::QueryResponse(
            QueryResponse::NodeDiagnostics(response),
        ))
        .expect("host response must serialize");

        assert!(
            whole.ends_with(&inner),
            "NodeDiagnosticsResponse must remain the LAST thing in its encoding. \
             Something now follows it, so appending a field to it would no longer be \
             trailing-byte-safe for older clients — it would silently corrupt whatever \
             was added after it."
        );
    }

    /// The rule above, in code that has already shipped.
    ///
    /// `ContractState::size_bytes` was appended in #52 (2026-02-18, crate
    /// version 0.1.36) and is present in every released tag from `rust-v0.8.0`
    /// onward. `ContractState` is a `HashMap` **value** inside
    /// `NodeDiagnosticsResponse`, and two more fields follow the map — so the
    /// appended `u64` is not trailing. It shifts everything after it.
    ///
    /// A client built before that commit, querying a node built after it, does
    /// not get a diagnostics response with one field missing. It gets **no
    /// diagnostics response at all**: the decoder reads the appended `u64` as
    /// the next value in sequence and fails, or worse, does not.
    ///
    /// This test isolates that single variable — it uses today's `String` map
    /// key, so it measures the effect of the appended field and not of the
    /// later key change in #70.
    ///
    /// Nothing here is fixable after the fact; the released bytes are the
    /// released bytes. It is pinned as the concrete instance of why the table
    /// on this module matters, and because a rule with a real example attached
    /// is the one people believe.
    #[test]
    fn the_shipped_size_bytes_append_is_an_instance_of_this() {
        use super::{
            ConnectedPeerInfo, ContractState, NetworkInfo, NodeInfo, SubscriptionInfo,
            SystemMetrics,
        };
        use crate::contract_interface::ContractInstanceId;
        use std::collections::HashMap;

        /// `ContractState` as it was before #52 appended `size_bytes`.
        #[allow(dead_code)] // decoded into, never read — the decode is the test
        #[derive(Serialize, Deserialize, Debug)]
        struct OldContractState {
            subscribers: u32,
            subscriber_peer_ids: Vec<String>,
        }

        /// `NodeDiagnosticsResponse` as an older client sees it: identical in
        /// every respect except the map's value type.
        #[allow(dead_code)]
        #[derive(Serialize, Deserialize, Debug)]
        struct OldNodeDiagnosticsResponse {
            node_info: Option<NodeInfo>,
            network_info: Option<NetworkInfo>,
            subscriptions: Vec<SubscriptionInfo>,
            contract_states: HashMap<String, OldContractState>,
            system_metrics: Option<SystemMetrics>,
            connected_peers_detailed: Vec<ConnectedPeerInfo>,
        }

        let mut contract_states = HashMap::new();
        contract_states.insert(
            "6kVs66bKaQAC6ohr8b43SvJ95r36tc2hnG7HezmaJHF9".to_string(),
            ContractState {
                subscribers: 3,
                subscriber_peer_ids: vec!["peer-a".to_string()],
                size_bytes: 1024,
            },
        );

        let new_node_response = NodeDiagnosticsResponse {
            node_info: None,
            network_info: None,
            subscriptions: vec![SubscriptionInfo {
                contract_key: ContractInstanceId::new([7u8; 32]),
                client_id: 42,
            }],
            contract_states,
            system_metrics: Some(SystemMetrics {
                active_connections: 1,
                hosting_contracts: 1,
            }),
            connected_peers_detailed: vec![ConnectedPeerInfo {
                peer_id: "peer-x".to_string(),
                address: "10.0.0.1:31337".to_string(),
            }],
        };

        let bytes = bincode::serialize(&new_node_response).expect("a new node's response");

        let faithfully_decoded = match bincode::deserialize::<OldNodeDiagnosticsResponse>(&bytes) {
            // The common outcome, and the honest one: the shifted bytes do not
            // form a decodable value and the client sees an error.
            Err(_) => false,
            // The quieter outcome: it decodes into something, and that
            // something is wrong.
            Ok(decoded) => {
                decoded.system_metrics.is_some() && decoded.connected_peers_detailed.len() == 1
            }
        };

        assert!(
            !faithfully_decoded,
            "an appended field on a non-terminal struct must not decode cleanly on an older \
             reader; if this ever passes, bincode gained field framing and the compatibility \
             table on this module needs rewriting"
        );
    }
}
