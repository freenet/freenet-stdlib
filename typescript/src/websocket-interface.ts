import * as flatbuffers from "flatbuffers";
import base58 from "bs58";

import { CHUNK_SIZE, CHUNK_THRESHOLD, ReassemblyBuffer } from "./streaming";
import { ContractContainerT } from "./common/contract-container";
import { ContractInstanceIdT } from "./common/contract-instance-id";
import { DeltaUpdateT } from "./common/delta-update";
import { RelatedDeltaUpdateT } from "./common/related-delta-update";
import { RelatedStateAndDeltaUpdateT } from "./common/related-state-and-delta-update";
import { RelatedStateUpdateT } from "./common/related-state-update";
import { StateAndDeltaUpdateT } from "./common/state-and-delta-update";
import { StateUpdateT } from "./common/state-update";
import {
  ApplicationMessagesT,
  AuthenticateT,
  ClientRequest,
  ClientRequestT,
  ClientRequestType,
  ContractRequestT,
  ContractRequestType,
  DelegateCodeT,
  DelegateContainerT,
  DelegateKeyT,
  DelegateRequestT,
  DelegateRequestType,
  DelegateType,
  DisconnectT,
  GetT,
  InboundDelegateMsgT,
  InboundDelegateMsgType,
  PutT,
  RegisterDelegateT,
  RelatedContractsT,
  SubscribeT,
  UnregisterDelegateT,
  UpdateT,
  UserInputResponseT,
  WasmDelegateV1T,
} from "./client-request";
import { StreamChunkT as ClientStreamChunkT } from "./client-request/stream-chunk";
import { UpdateDataT } from "./common/update-data";
import { UpdateDataType } from "./common/update-data-type";
export { UpdateDataType } from "./common/update-data-type";
export { ContractType } from "./common/contract-type";
import { ContractKeyT } from "./common/contract-key";
import { PutResponseT } from "./host-response/put-response";
import { GetResponseT } from "./host-response/get-response";
import { UpdateResponseT } from "./host-response/update-response";
import { UpdateNotificationT } from "./host-response/update-notification";
import { HostResponse, HostResponseT } from "./host-response/host-response";
import {
  ContextUpdatedT,
  ContractResponseT,
  ContractResponseType,
  DelegateResponseT,
  HostResponseType,
  OutboundDelegateMsgT,
  OutboundDelegateMsgType,
  RequestUserInputT,
  StreamChunkT,
  SubscribeResponseT,
} from "./host-response";
import {
  ApplicationMessageT,
  ContractCodeT,
  ContractType,
  WasmContractV1T,
} from "./common";
import { ErrorT } from "./host-response/error";
import { NotFoundT } from "./host-response/not-found";

// Common types
/**
 * The id of a live instance of a contract. This is effectively the tuple
 * of the hash of the hash of the contract code and a set of parameters used to run
 * the contract.
 * @public
 */
export type ContractInstanceId = Uint8Array;

/**
 * Update notifications for a contract or a related contract.
 * @public
 */
export class UpdateData extends UpdateDataT {
  constructor(
    updateDataType: UpdateDataType = UpdateDataType.NONE,
    updateData:
      | DeltaUpdateT
      | RelatedDeltaUpdateT
      | RelatedStateAndDeltaUpdateT
      | RelatedStateUpdateT
      | StateAndDeltaUpdateT
      | StateUpdateT
      | null = null
  ) {
    super(updateDataType, updateData);
  }
}

/**
 * Representation of the state update data
 * @public
 */
export class StateUpdate extends StateUpdateT {
  constructor(state: number[] = []) {
    super(state);
  }
}

/**
 * Representation of the delta update data
 * @public
 */
export class DeltaUpdate extends DeltaUpdateT {
  constructor(delta: number[] = []) {
    super(delta);
  }
}

/**
 * Representation of the state and delta update data
 * @public
 */
export class StateAndDeltaUpdate extends StateAndDeltaUpdateT {
  constructor(state: number[] = [], delta: number[] = []) {
    super(state, delta);
  }
}

/**
 * Representation of the related state update data
 * @public
 */
export class RelatedStateUpdate extends RelatedStateUpdateT {
  constructor(
    relatedTo: ContractInstanceIdT | null = null,
    state: number[] = []
  ) {
    super(relatedTo, state);
  }
}

/**
 * Representation of the related delta update data
 * @public
 */
export class RelatedDeltaUpdate extends RelatedDeltaUpdateT {
  constructor(
    relatedTo: ContractInstanceIdT | null = null,
    delta: number[] = []
  ) {
    super(relatedTo, delta);
  }
}

/**
 * Representation of the related state and delta update data
 * @public
 */
export class RelatedStateAndDeltaUpdate extends RelatedStateAndDeltaUpdateT {
  constructor(
    relatedTo: ContractInstanceIdT | null = null,
    state: number[] = [],
    delta: number[] = []
  ) {
    super(relatedTo, state, delta);
  }
}

/**
 * Representation of the ContractKey
 * @public
 */
export class ContractKey extends ContractKeyT {
  constructor(instance: ContractInstanceId, code?: Uint8Array) {
    if (instance.length !== 32 || (code && code.length !== 32)) {
      throw new TypeError("Invalid array length, expected 32 bytes");
    }

    let contract_instance_id = new ContractInstanceIdT(Array.from(instance));
    let contract_code: number[] = [];
    if (code) {
      contract_code = Array.from(code);
    }
    super(contract_instance_id, contract_code);
  }

  static fromInstanceId(spec: string): ContractKey {
    const decoded = base58.decode(spec);
    return new ContractKey(decoded);
  }

  bytes(): ContractInstanceId {
    return new Uint8Array(this.instance?.data!) as ContractInstanceId;
  }

  codePart(): Uint8Array | null {
    return new Uint8Array(this.code);
  }

  encode(): string {
    const instance = new Uint8Array(this.instance?.data!);
    return base58.encode(instance);
  }

  get_contract_key(): ContractKey {
    return this;
  }
}

/**
 * Representation of the DelegateKey
 * @public
 */
export type DelegateKey = DelegateKeyT;

/**
 * Representation of the ContractCode
 * @public
 */
export type ContractCode = ContractCodeT;

/**
 * Representation of the DelegateCode
 * @public
 */
export type DelegateCode = DelegateCodeT;

/**
 * Representation of the WasmContractV1
 * @public
 */
export class WasmContractV1 extends WasmContractV1T {
  constructor(
    data: ContractCode | null = null,
    parameters: number[] = [],
    key: ContractKeyT | null = null
  ) {
    super(data, parameters, key);
  }
}

/**
 * Contract version type
 */
export type Contract = WasmContractV1;

/**
 * Wrapper that allows contract versioning. This enum maintains the types of contracts that are allowed
 * and their corresponding version.
 * @public
 */
export class ContractContainer extends ContractContainerT {
  constructor(
    contractType: ContractType = ContractType.NONE,
    contract: Contract
  ) {
    super(contractType, contract);
  }
}

/**
 * Representation of the WasmDelegateV1
 * @public
 */
export class WasmDelegateV1 extends WasmDelegateV1T {
  constructor(parameters: number[] = [], data: DelegateCode, key: DelegateKey) {
    super(parameters, data, key);
  }
}

/**
 * Delegate version type
 */
export type Delegate = WasmDelegateV1;

/**
 * Wrapper that allows delegate versioning. This enum maintains the types of delegates that are allowed
 * and their corresponding version.
 * @public
 */
export class DelegateContainer extends DelegateContainerT {
  constructor(
    delegateType: DelegateType = DelegateType.NONE,
    delegate: Delegate
  ) {
    super(delegateType, delegate);
  }
}

/**
 * Representation of the delegate Application message
 *
 */
export type ApplicationMessage = ApplicationMessageT;

// Client requests

// Contract

/**
 * Representation of the client put request operation
 * @public
 */
export class PutRequest extends PutT {
  constructor(
    container: ContractContainerT | null = null,
    wrappedState: number[] = [],
    relatedContracts: RelatedContractsT | null = null,
    subscribe: boolean = false,
    blockingSubscribe: boolean = false
  ) {
    super(container, wrappedState, relatedContracts, subscribe, blockingSubscribe);
  }
}

/**
 * Representation of the client update request operation
 * @public
 */
export class UpdateRequest extends UpdateT {
  constructor(
    key: ContractKey | null = null,
    update: UpdateDataT | null = null
  ) {
    const contract_key = key?.get_contract_key();
    super(contract_key, update);
  }
}

/**
 * Representation of the client get request operation
 * @public
 */
export class GetRequest extends GetT {
  constructor(key: ContractKey, fetchContract: boolean = false, subscribe: boolean = false, blockingSubscribe: boolean = false) {
    const contract_key = key.get_contract_key();
    super(contract_key, fetchContract, subscribe, blockingSubscribe);
  }
}

/**
 * Representation of the client subscribe request operation
 * @public
 */
export class SubscribeRequest extends SubscribeT {
  constructor(key: ContractKey | null = null, summary: number[] = []) {
    const contract_key = key?.get_contract_key();
    super(contract_key, summary);
  }
}

/**
 * Representation of the client disconnect request operation
 * @public
 */
export class DisconnectRequest extends DisconnectT {
  constructor(cause: string | Uint8Array | null = null) {
    super(cause);
  }
}

// Delegate
/**
 * Representation of the UserInputResponse message
 * @public
 */
export type UserInputResponse = UserInputResponseT;

export type InboundMessage =
  | ApplicationMessage
  | UserInputResponse;

/**
 * Representation of DelegateRequest Inbound message
 * @public
 */
export class InboundDelegateMsg extends InboundDelegateMsgT {
  constructor(
    inboundType: InboundDelegateMsgType = InboundDelegateMsgType.NONE,
    inbound: InboundMessage
  ) {
    super(inboundType, inbound);
  }
}
/**
 * Representation of an inbound application messages
 * @public
 */
export type ApplicationMessages = ApplicationMessagesT;
/**
 * Representation of the RegisterDelegate message
 * @public
 */
export type RegisterDelegate = RegisterDelegateT;
/**
 * Representation of the UnregisterDelegate message
 * @public
 */
export type UnregisterDelegate = UnregisterDelegateT;

export class DelegateRequest extends DelegateRequestT {
  constructor(
    delegateRequestType: DelegateRequestType = DelegateRequestType.NONE,
    delegateRequest:
      | ApplicationMessages
      | RegisterDelegate
      | UnregisterDelegate
  ) {
    super(delegateRequestType, delegateRequest);
  }
}

// Host replies

// Contract
/**
 * The response for a contract put operation
 * @public
 */
export class PutResponse extends PutResponseT {
  constructor(public key: ContractKey) {
    super(key);
  }

  static fromPutResponseT(obj: PutResponseT): PutResponse {
    // Build the contract key
    let instance = new Uint8Array(obj.key?.instance?.data!);
    const code =
      obj.key?.code && obj.key.code.length > 0
        ? new Uint8Array(obj.key.code!)
        : undefined;
    let key: ContractKey = new ContractKey(instance, code);

    return new PutResponse(key);
  }
}

/**
 * The response for a contract get operation
 * @public
 */
export class GetResponse extends GetResponseT {
  constructor(
    public key: ContractKey,
    public contract: ContractContainer,
    public state: number[] = []
  ) {
    super(key, contract, state);
  }

  static fromGetResponseT(obj: GetResponseT): GetResponse {
    // Build the contract key
    let instance = new Uint8Array(obj.key?.instance?.data!);
    const code =
      obj.key?.code && obj.key.code.length > 0
        ? new Uint8Array(obj.key.code!)
        : undefined;
    let key: ContractKey = new ContractKey(instance, code);

    return new GetResponse(key, obj.contract!, obj.state);
  }
}

/**
 * The response for a contract update operation
 * @public
 */
export class UpdateResponse extends UpdateResponseT {
  constructor(public key: ContractKey, public summary: number[] = []) {
    super(key, summary);
  }

  static fromUpdateResponseT(obj: UpdateResponseT): UpdateResponse {
    // Build the contract key
    let instance = new Uint8Array(obj.key?.instance?.data!);
    const code =
      obj.key?.code && obj.key.code.length > 0
        ? new Uint8Array(obj.key.code!)
        : undefined;
    let key: ContractKey = new ContractKey(instance, code);

    return new UpdateResponse(key, obj.summary);
  }
}

/**
 * The response for a contract update notification
 * @public
 */
export class UpdateNotification extends UpdateNotificationT {
  constructor(public key: ContractKey, public update: UpdateData) {
    super(key, update);
  }

  static fromUpdateNotificationT(obj: UpdateNotificationT): UpdateNotification {
    // Build the contract key
    let instance = new Uint8Array(obj.key?.instance?.data!);
    const code =
      obj.key?.code && obj.key.code.length > 0
        ? new Uint8Array(obj.key.code!)
        : undefined;
    let key: ContractKey = new ContractKey(instance, code);

    return new UpdateNotification(key, obj.update!);
  }
}

// Delegate
/**
 * Representation of ContextUpdated message
 * @public
 */
export type ContextUpdated = ContextUpdatedT;
/**
 * Representation of RequestUserInput message
 * @public
 */
export type RequestUserInput = RequestUserInputT;

/**
 * Representation of the outbound delegate message types
 * @public
 */
export type OutboundMessage =
  | ApplicationMessage
  | RequestUserInput
  | ContextUpdated;

export class OutboundDelegateMsg extends OutboundDelegateMsgT {
  constructor(
    inboundType: OutboundDelegateMsgType = OutboundDelegateMsgType.NONE,
    inbound: OutboundMessage
  ) {
    super(inboundType, inbound);
  }
}

/**
 * The response for a delegate operation
 * @public
 */
export class DelegateResponse extends DelegateResponseT {
  constructor(
    key: DelegateKey | null = null,
    values: OutboundDelegateMsg[] = []
  ) {
    super(key, values);
  }
}

/**
 * Host response error type
 * @public
 */
export type HostError = {
  cause: string;
};

// API

/**
 * Interface to handle responses from the host
 *
 * @example
 * Here's a simple implementation example:
 * ```
 * const handler = {
 *  onContractPut: (_response: PutResponse) => {},
 *  onContractGet: (_response: GetResponse) => {},
 *  onContractUpdate: (_up: UpdateResponse) => {},
 *  onContractUpdateNotification: (_notif: UpdateNotification) => {},
 *  onDelegateResponse: (_response: DelegateResponse) => {},
 *  onErr: (err: HostError) => {},
 *  onOpen: () => {},
 * };
 * ```
 *
 * @public
 */
export interface ResponseHandler {
  /**
   * Contract `Put` response handler
   */
  onContractPut: (response: PutResponse) => void;
  /**
   * Contract `Get` response handler
   */
  onContractGet: (response: GetResponse) => void;
  /**
   * Contract `Update` response handler
   */
  onContractUpdate: (response: UpdateResponse) => void;
  /**
   * Contract `Update` notification handler
   */
  onContractUpdateNotification: (response: UpdateNotification) => void;
  /**
   * Contract `NotFound` handler
   */
  onContractNotFound: (instanceId: ContractInstanceId) => void;
  /**
   * Contract `Subscribe` confirmation handler
   */
  onSubscribeResponse?: (key: ContractKey, subscribed: boolean) => void;
  /**
   * `Delegate` response handler
   * @param response
   */
  onDelegateResponse: (response: DelegateResponse) => void;
  /**
   * Contract `Error` handler
   */
  onErr: (response: HostError) => void;
  /**
   * Callback executed after successfully establishing connection with websocket
   */
  onOpen: () => void;
  /**
   * Called when WebSocket connection closes
   */
  onClose?: (code: number, reason: string) => void;
}

const ENCODING_PROTOC: string = "flatbuffers";

function getAuthTokenFromCookie(): string | null {
  if (typeof document === "undefined") return null;
  const cookies = document.cookie.split(";");
  for (let cookie of cookies) {
    const [cookieName, cookieValue] = cookie.trim().split("=");
    if (cookieName === "authorization") {
      const authString = decodeURIComponent(cookieValue).split("Bearer ");
      if (authString.length == 2) {
        return authString[1];
      }
      return null;
    }
  }
  return null;
}

/**
 * Resolves a WebSocket constructor for the current environment.
 * Uses the global `WebSocket` in browsers, falls back to the `ws` package in Node.js.
 */
function resolveWebSocket(): typeof WebSocket {
  if (typeof WebSocket !== "undefined") return WebSocket;
  try {
    // Node.js — require ws at runtime so it's not bundled in browsers
    return require("ws") as typeof WebSocket;
  } catch {
    throw new Error(
      "No WebSocket implementation found. Install the 'ws' package for Node.js support."
    );
  }
}

interface PendingRequest<T> {
  resolve: (value: T) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
  /**
   * base58 contract instance id this request is correlated on. See
   * {@link takeMatching}.
   *
   * `null` only if the key could not be read off the outgoing request, which
   * the schema makes unreachable through the typed API: `key` is `(required)`
   * on `Get`, `Update`, `Subscribe` and `WasmContractV1` (client_request.fbs,
   * common.fbs), so `sendRequest` throws while packing before the request is
   * ever queued. A `null`-key entry therefore matches nothing and would settle
   * by timeout; there is deliberately no fall-back tier for it.
   */
  key: string | null;
}

/** Default timeout for awaiting a response (30 seconds). */
const REQUEST_TIMEOUT_MS = 30_000;

/** How many abandoned request keys to remember. See `rememberAbandoned`. */
const ABANDONED_KEY_MEMORY = 64;

/** The base58 alphabet, as a single-character test. */
const BASE58_CHAR = /[1-9A-HJ-NP-Za-km-z]/;

/**
 * Whether `message` names `key` as a whole base58 token.
 *
 * A plain substring test is not an identity test here. Base58 encodes each
 * leading zero byte of an instance id as a leading `'1'`, so ids with a long
 * run of leading zeros produce short, `'1'`-heavy strings, and one key's full
 * encoding can be a literal prefix of another's. Requiring that the match is
 * not butted up against another base58 character makes a prefix relationship
 * stop counting as a mention.
 */
function mentionsKey(message: string, key: string): boolean {
  for (let from = 0; ; from += 1) {
    const at = message.indexOf(key, from);
    if (at === -1) return false;
    const before = at === 0 ? "" : message[at - 1];
    const after = message[at + key.length] ?? "";
    if (!BASE58_CHAR.test(before) && !BASE58_CHAR.test(after)) return true;
    from = at;
  }
}

/**
 * base58-encodes a contract instance id, or returns `null` when the id is
 * missing or not 32 bytes (so a malformed key degrades to "uncorrelated"
 * rather than throwing on the response path).
 */
function encodeInstanceId(
  data: number[] | Uint8Array | null | undefined
): string | null {
  if (!data) return null;
  const bytes = data instanceof Uint8Array ? data : Uint8Array.from(data);
  if (bytes.length !== 32) return null;
  return base58.encode(bytes);
}

/** base58 instance id of a wire `ContractKey`, or `null` when unusable. */
function instanceIdOf(key: ContractKeyT | null | undefined): string | null {
  return encodeInstanceId(key?.instance?.data);
}

/**
 * base58 instance id a `Put` request expects back.
 *
 * The node recomputes a put's key from the contract code and parameters
 * (`ContractKey::from_params_and_code`), so the key embedded in the container
 * is what the *client* believes it is putting. It is the only correlation
 * material a put carries; {@link FreenetWsApi.handleResponse} falls back to the
 * lone in-flight put when the host disagrees.
 */
function putRequestKey(put: PutT): string | null {
  return instanceIdOf(put.container?.contract?.key);
}

/**
 * The `FreenetWsApi` provides the API to manage the connection to the host,
 * handle responses, and send requests.
 *
 * Two APIs coexist and both fire for every response. The `ResponseHandler`
 * callbacks see everything the host sends, including update notifications and
 * responses nothing is waiting on. The promise-returning methods ({@link get},
 * {@link put}, {@link update}, {@link subscribe}) additionally settle the one
 * request a response belongs to.
 *
 * Responses are matched to requests **by contract key**, not by arrival order.
 * The node drives each contract operation on its own task and publishes results
 * as they complete, so a request issued first can be answered second; matching
 * by position would hand one request's answer to another. One response settles
 * every pending request for its key, since concurrent requests for one contract
 * are indistinguishable on the wire. See {@link takeMatching}.
 *
 * @example
 * Here's a simple example:
 * ```
 * const API_URL = new URL(`ws://${location.host}/contract/command/`);
 * const freenetApi = new FreenetWsApi(API_URL, handler);
 * ```
 */
export class FreenetWsApi {
  private ws: WebSocket;
  private responseHandler: ResponseHandler;
  private reassembly: ReassemblyBuffer = new ReassemblyBuffer();
  private nextStreamId = 0;
  private pendingGets: PendingRequest<GetResponse>[] = [];
  private pendingPuts: PendingRequest<PutResponse>[] = [];
  private pendingUpdates: PendingRequest<UpdateResponse>[] = [];
  private pendingSubscribes: PendingRequest<void>[] = [];
  /**
   * Keys of requests that left a queue without ever getting their own answer —
   * timed out, or failed by a connection-wide error. A late response for one of
   * these must never be handed to a different request. See
   * {@link rememberAbandoned} and the lone-put fall-back in
   * {@link takeMatching}.
   */
  private abandonedKeys: string[] = [];

  /**
   * @constructor
   * @param url - The websocket URL to establish the connection.
   * @param handler - The ResponseHandler implementation
   * @param authToken - Optional auth token (falls back to browser cookie)
   */
  constructor(url: URL, handler: ResponseHandler, authToken?: string) {
    this.responseHandler = handler;
    const token = authToken ?? getAuthTokenFromCookie();
    if (token) {
      url.searchParams.append("authToken", token);
    }
    url.searchParams.append("encodingProtocol", ENCODING_PROTOC);
    const WS = resolveWebSocket();
    this.ws = new WS(url.toString());
    this.ws.binaryType = "arraybuffer";
    this.ws.onmessage = (ev) => this.handleResponse(ev);
    this.ws.addEventListener("open", () => {
      if (authToken) {
        this.sendRequest(new ClientRequestT(
          ClientRequestType.Authenticate,
          new AuthenticateT(authToken)
        ));
      }
      handler.onOpen();
    });
    this.ws.addEventListener("close", (ev: CloseEvent) => {
      this.rejectAllPending(new Error(`Connection closed: ${ev.reason || ev.code}`));
      handler.onClose?.(ev.code, ev.reason);
    });
  }

  /**
   * @private
   */
  private handleResponse(ev: MessageEvent<any>): void | Error {
    let response: HostResponseT;
    try {
      let data = new flatbuffers.ByteBuffer(new Uint8Array(ev.data));
      response = HostResponse.getRootAsHostResponse(data).unpack();
    } catch (err) {
      console.log(`found error: ${err}`);
      return new Error(`${err}`);
    }
    switch (response.responseType) {
      case HostResponseType.ContractResponse:
        let host_resp = response.response as ContractResponseT;
        switch (host_resp.contractResponseType) {
          case ContractResponseType.PutResponse:
            const put_response = PutResponse.fromPutResponseT(
              host_resp.contractResponse as PutResponseT
            );
            this.responseHandler.onContractPut(put_response);
            // `allowLoneFallback`: the host's key is authoritative and may
            // differ from the container key the client supplied, so a single
            // in-flight put still claims the response. With two or more in
            // flight the answer would be a guess, and a guess here is the
            // silent data mix-up this correlation exists to prevent.
            this.resolveMatching(
              this.pendingPuts,
              put_response.key.encode(),
              put_response,
              true
            );
            break;
          case ContractResponseType.GetResponse:
            const get_response = GetResponse.fromGetResponseT(
              host_resp.contractResponse as GetResponseT
            );
            this.responseHandler.onContractGet(get_response);
            this.resolveMatching(
              this.pendingGets,
              get_response.key.encode(),
              get_response
            );
            break;
          case ContractResponseType.UpdateResponse:
            const update_response = UpdateResponse.fromUpdateResponseT(
              host_resp.contractResponse as UpdateResponseT
            );
            this.responseHandler.onContractUpdate(update_response);
            this.resolveMatching(
              this.pendingUpdates,
              update_response.key.encode(),
              update_response
            );
            break;
          case ContractResponseType.UpdateNotification:
            const update_notification =
              UpdateNotification.fromUpdateNotificationT(
                host_resp.contractResponse as UpdateNotificationT
              );
            this.responseHandler.onContractUpdateNotification(
              update_notification
            );
            break;
          case ContractResponseType.NotFound:
            const not_found = host_resp.contractResponse as NotFoundT;
            const not_found_id = new Uint8Array(not_found.instanceId?.data ?? []);
            this.responseHandler.onContractNotFound(not_found_id);
            // `NotFound` carries only the instance id (host_response.fbs), which
            // is exactly what requests are correlated on.
            //
            // Only `pendingGets` is failed. A pending `subscribe()` for the same
            // contract is deliberately left alone: the host answers a subscribe
            // for a missing contract with an `Error` naming the contract, which
            // `rejectForError` attributes, so the caller still fails fast rather
            // than waiting out REQUEST_TIMEOUT_MS. If that ever stops holding,
            // fail `pendingSubscribes` here too.
            this.rejectMatching(
              this.pendingGets,
              encodeInstanceId(not_found_id),
              new Error("Contract not found")
            );
            break;
          case ContractResponseType.SubscribeResponse:
            const sub_resp = host_resp.contractResponse as SubscribeResponseT;
            const sub_instance = new Uint8Array(sub_resp.key?.instance?.data ?? []);
            const sub_code = sub_resp.key?.code && sub_resp.key.code.length > 0
              ? new Uint8Array(sub_resp.key.code)
              : undefined;
            const sub_key = new ContractKey(sub_instance, sub_code);
            this.responseHandler.onSubscribeResponse?.(sub_key, sub_resp.subscribed);
            if (sub_resp.subscribed) {
              this.resolveMatching(
                this.pendingSubscribes,
                sub_key.encode(),
                undefined
              );
            } else {
              this.rejectMatching(
                this.pendingSubscribes,
                sub_key.encode(),
                new Error(
                  `Host reported contract ${sub_key.encode()} as not subscribed`
                )
              );
            }
            break;
          default:
            const cause = "Contract response type not implemented";
            console.log(cause);
            const err: HostError = {
              cause,
            };
            this.responseHandler.onErr(err);
            break;
        }
        break;
      case HostResponseType.DelegateResponse:
        let delegate_response = response.response as DelegateResponseT;
        this.responseHandler.onDelegateResponse(delegate_response);
        break;
      case HostResponseType.Ok:
        break;
      case HostResponseType.Error:
        const error_resp = response.response as ErrorT;
        const error_msg = error_resp.msg;
        const error_cause = typeof error_msg === "string"
          ? error_msg
          : error_msg instanceof Uint8Array
            ? new TextDecoder().decode(error_msg)
            : "unknown error";
        const host_error: HostError = { cause: error_cause };
        this.responseHandler.onErr(host_error);
        this.rejectForError(error_cause);
        break;
      case HostResponseType.StreamChunk: {
        const streamChunk = response.response as StreamChunkT;
        try {
          const assembled = this.reassembly.receiveChunk(
            streamChunk.streamId,
            streamChunk.index,
            streamChunk.total,
            new Uint8Array(streamChunk.data)
          );
          if (assembled !== null) {
            // Reassembly complete — re-dispatch the inner response
            const syntheticEvent = { data: assembled.buffer } as MessageEvent;
            this.handleResponse(syntheticEvent);
          }
        } catch (err) {
          const streamErr: HostError = {
            cause: `Stream reassembly error: ${err}`,
          };
          this.responseHandler.onErr(streamErr);
          if (streamChunk.streamId !== undefined) {
            this.reassembly.removeStream(streamChunk.streamId);
          }
        }
        break;
      }
      default:
        const cause = `Received wrong HostResponse type`;
        console.log(cause);
        const err: HostError = {
          cause,
        };
        this.responseHandler.onErr(err);
        break;
    }
  }

  /**
   * Serializes a ClientRequest and sends it over the WebSocket.
   * Automatically chunks payloads exceeding CHUNK_THRESHOLD.
   */
  private sendRequest(request: ClientRequestT): void {
    const fbb = new flatbuffers.Builder(1024);
    ClientRequest.finishClientRequestBuffer(fbb, request.pack(fbb));
    const bytes = fbb.asUint8Array();
    if (bytes.byteLength > CHUNK_THRESHOLD) {
      this.sendChunked(bytes);
    } else {
      this.ws.send(bytes);
    }
  }

  /**
   * Splits a serialized payload into StreamChunk messages and sends each.
   */
  private sendChunked(payload: Uint8Array): void {
    const streamId = this.nextStreamId++;
    const total = Math.ceil(payload.byteLength / CHUNK_SIZE);
    for (let i = 0; i < total; i++) {
      const start = i * CHUNK_SIZE;
      const end = Math.min(start + CHUNK_SIZE, payload.byteLength);
      const chunk = new ClientStreamChunkT(
        streamId,
        i,
        total,
        Array.from(payload.subarray(start, end))
      );
      const request = new ClientRequestT(ClientRequestType.StreamChunk, chunk);
      const fbb = new flatbuffers.Builder(end - start + 128);
      ClientRequest.finishClientRequestBuffer(fbb, request.pack(fbb));
      this.ws.send(fbb.asUint8Array());
    }
  }

  /**
   * Enqueues a pending response and returns a promise that resolves
   * when the matching response arrives, or rejects on timeout.
   */
  private awaitResponse<T>(
    queue: PendingRequest<T>[],
    key: string | null
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        const idx = queue.findIndex((p) => p.timer === timer);
        if (idx !== -1) queue.splice(idx, 1);
        this.rememberAbandoned(key);
        reject(new Error("Request timeout"));
      }, REQUEST_TIMEOUT_MS);
      queue.push({ resolve, reject, timer, key });
    });
  }

  /** Every queue of awaited responses, for connection-wide failures. */
  private get allPendingQueues(): PendingRequest<any>[][] {
    return [
      this.pendingGets,
      this.pendingPuts,
      this.pendingUpdates,
      this.pendingSubscribes,
    ];
  }

  /**
   * Removes and returns every pending request a response settles.
   *
   * Contract operations carry no request id on the wire — the only identifying
   * field a response has is the contract key (`ContractResponse` in
   * host_response.fbs), so that is what requests are correlated on. Responses
   * arrive in completion order, not request order, because the node drives each
   * operation on its own task, so matching by position would hand one request's
   * answer to another.
   *
   * In order:
   * 1. every request awaiting this exact key;
   * 2. only when `allowLoneFallback` is set, exactly one request is in flight,
   *    and this key is not one we have abandoned — that request; see the put
   *    call site for why the tier exists and {@link rememberAbandoned} for why
   *    it is fenced;
   * 3. otherwise nothing: an unmatched response is dropped rather than
   *    mis-delivered, and the requests keep waiting for their own answer (or
   *    for REQUEST_TIMEOUT_MS).
   *
   * Tier 1 settles *all* same-key requests, not just the oldest, because the
   * client cannot tell them apart — the wire has no request id, and freenet-core
   * drops its internal `RequestId` before `ClientEventsProxy::send`. Settling
   * one and leaving the rest queued is not a safe default: the node coalesces
   * byte-identical concurrent UPDATEs into a single transaction (its
   * `RequestRouter` dedup) and emits one result for both, so the extra callers
   * would wait out REQUEST_TIMEOUT_MS for an answer that is never coming twice.
   * This is a trade, not a free win. Two same-key requests with genuinely
   * different outcomes both take the first outcome, and what that costs depends
   * on the operation. For a get it is benign — same contract, same answer
   * either way. For an update it is what the node's own coalescing already
   * does. For a **put it is a real mis-report**: two concurrent puts of
   * different state to one key mean the second caller is told its put succeeded
   * when in fact the first one's did. It is still the better trade — the wire
   * carries no request id, so there is nothing to tell the two apart, and a 30s
   * hang on a legitimate call is worse than an ambiguous success — but a reader
   * changing this should know the cost is real and lands on put.
   */
  private takeMatching<T>(
    queue: PendingRequest<T>[],
    key: string | null,
    allowLoneFallback = false
  ): PendingRequest<T>[] {
    let taken: PendingRequest<T>[] = [];
    if (key !== null) {
      taken = queue.filter((p) => p.key === key);
      for (const pending of taken) queue.splice(queue.indexOf(pending), 1);
    }
    if (
      taken.length === 0 &&
      allowLoneFallback &&
      queue.length === 1 &&
      !(key !== null && this.abandonedKeys.includes(key))
    ) {
      taken = queue.splice(0, 1);
    }
    for (const pending of taken) clearTimeout(pending.timer);
    return taken;
  }

  /**
   * Remembers that a request for `key` left a queue without ever getting its
   * own answer, so a late response for it can never be handed to some other
   * request by the lone-put fall-back.
   *
   * Without this, the fall-back cannot tell "the key differs because the host
   * recomputed it" — the case it exists for — from "the key differs because
   * this answer belongs to a request that is already gone". Put A times out and
   * is spliced out, the caller retries as put B, A's late answer arrives, and B
   * resolves with A's response while B's real answer is later dropped against
   * an empty queue. The conditions that cause a timeout are the same ones that
   * make a late arrival likely, so this is not a remote corner.
   *
   * A bounded FIFO rather than an expiring record: nothing here needs to be
   * exact, and a timer per entry would be another thing to leak. Sixty-four is
   * far more abandonments than a healthy connection produces, and an entry
   * ageing out only restores the previous behaviour for an answer that is by
   * then extremely late.
   */
  private rememberAbandoned(key: string | null): void {
    if (key === null || this.abandonedKeys.includes(key)) return;
    this.abandonedKeys.push(key);
    if (this.abandonedKeys.length > ABANDONED_KEY_MEMORY) {
      this.abandonedKeys.shift();
    }
  }

  /**
   * Resolves every pending request this response settles.
   */
  private resolveMatching<T>(
    queue: PendingRequest<T>[],
    key: string | null,
    value: T,
    allowLoneFallback = false
  ): void {
    for (const pending of this.takeMatching(queue, key, allowLoneFallback)) {
      pending.resolve(value);
    }
  }

  /**
   * Rejects every pending request this response settles.
   */
  private rejectMatching<T>(
    queue: PendingRequest<T>[],
    key: string | null,
    error: Error
  ): void {
    for (const pending of this.takeMatching(queue, key)) pending.reject(error);
  }

  /**
   * Fails the requests a host `Error` response refers to.
   *
   * The wire `Error` carries only a message (host_response.fbs), so there is
   * nothing to correlate on directly. freenet-stdlib renders the contract key
   * into that message (`ContractError`'s `Display`, as base58 of the instance
   * id), so this scans the message for the keys of the requests *we* are
   * waiting on, as whole base58 tokens ({@link mentionsKey}). Matching on our
   * own keys rather than parsing the host's sentence means a change to the
   * message wording degrades to the fall-back below instead of mis-attributing
   * the failure.
   *
   * When no pending key appears in the message the error cannot be attributed —
   * it may be connection-wide — so every pending request is failed, as before.
   * That keeps a genuinely fatal error from leaving callers hanging for the
   * full timeout.
   *
   * Residual imprecision, deliberately accepted: every in-flight request naming
   * that contract fails, across operations (a get and an update on the same
   * contract both fail) and across duplicates (two concurrent gets on it both
   * fail). Bounding the blast radius to one contract is the point; the host says
   * neither which operation failed nor which of two identical requests, so
   * failing all of them beats stranding the ones that are not the oldest.
   */
  private rejectForError(cause: string): void {
    const error = new Error(cause);
    let matched = false;
    for (const queue of this.allPendingQueues) {
      const key = queue.find(
        (p) => p.key !== null && mentionsKey(cause, p.key)
      )?.key;
      if (key === undefined) continue;
      matched = true;
      this.rejectMatching(queue, key, error);
    }
    if (!matched) this.rejectAllPending(error);
  }

  /**
   * Rejects all pending requests across all queues.
   */
  private rejectAllPending(error: Error): void {
    for (const queue of this.allPendingQueues) {
      while (queue.length > 0) {
        const pending = queue.shift()!;
        clearTimeout(pending.timer);
        // Same reasoning as the timeout path: these never got their own answer,
        // so a late one must not be handed to a later request.
        this.rememberAbandoned(pending.key);
        pending.reject(error);
      }
    }
  }

  /**
   * Sends a put request and returns the response.
   * @param put - The `PutRequest` object
   */
  async put(put: PutRequest): Promise<PutResponse> {
    this.sendRequest(new ClientRequestT(
      ClientRequestType.ContractRequest,
      new ContractRequestT(ContractRequestType.Put, put)
    ));
    return this.awaitResponse(this.pendingPuts, putRequestKey(put));
  }

  /**
   * Sends an update request and returns the response.
   * @param update - The `UpdateRequest` object
   */
  async update(update: UpdateRequest): Promise<UpdateResponse> {
    this.sendRequest(new ClientRequestT(
      ClientRequestType.ContractRequest,
      new ContractRequestT(ContractRequestType.Update, update)
    ));
    return this.awaitResponse(this.pendingUpdates, instanceIdOf(update.key));
  }

  /**
   * Sends a get request and returns the response.
   * @param get - The `GetRequest` object
   */
  async get(get: GetRequest): Promise<GetResponse> {
    this.sendRequest(new ClientRequestT(
      ClientRequestType.ContractRequest,
      new ContractRequestT(ContractRequestType.Get, get)
    ));
    return this.awaitResponse(this.pendingGets, instanceIdOf(get.key));
  }

  /**
   * Sends a subscribe request and waits for the host to confirm it.
   *
   * Resolves once the host answers with `SubscribeResponse { subscribed: true }`
   * for this contract. Rejects if the host answers `subscribed: false` (for
   * instance when a subscriber limit is reached), reports an error naming the
   * contract, closes the connection, or does not answer within
   * REQUEST_TIMEOUT_MS.
   *
   * @param subscribe - The `SubscribeRequest` object
   */
  async subscribe(subscribe: SubscribeRequest): Promise<void> {
    this.sendRequest(new ClientRequestT(
      ClientRequestType.ContractRequest,
      new ContractRequestT(ContractRequestType.Subscribe, subscribe)
    ));
    return this.awaitResponse(
      this.pendingSubscribes,
      instanceIdOf(subscribe.key)
    );
  }

  /**
   * Sends a disconnect notification to the host through websocket.
   * @param disconnect - The `DisconnectRequest` object
   */
  async disconnect(disconnect: DisconnectRequest): Promise<void> {
    this.sendRequest(new ClientRequestT(ClientRequestType.Disconnect, disconnect));
  }
}
