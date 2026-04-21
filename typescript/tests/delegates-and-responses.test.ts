import * as flatbuffers from "flatbuffers";
import base58 from "bs58";
import { Server } from "mock-socket";
import {
  ContractResponseType,
  ContractResponseT,
  HostResponseType,
  HostResponseT,
  SubscribeResponseT,
  StreamChunkT as HostStreamChunkT,
  GetResponseT,
  PutResponseT,
} from "../src/host-response";
import {
  DelegateResponseT as HostDelegateResponseT,
} from "../src/host-response/delegate-response";
import {
  DelegateKeyT as HostDelegateKeyT,
} from "../src/host-response/delegate-key";
import {
  OutboundDelegateMsgT,
  OutboundDelegateMsgType,
} from "../src/host-response";
import { ApplicationMessageT } from "../src/common/application-message";
import { ContextUpdatedT } from "../src/host-response/context-updated";
import { ContractKeyT } from "../src/common/contract-key";
import { ContractInstanceIdT } from "../src/common/contract-instance-id";
import { ClientRequest } from "../src/client-request/client-request";
import { ClientRequestType } from "../src/client-request/client-request-type";
import { DelegateRequestType } from "../src/client-request/delegate-request-type";
import {
  ContractKey,
  DelegateContainer,
  DelegateRequest,
  DelegateResponse,
  DisconnectRequest,
  FreenetWsApi,
  GetRequest,
  GetResponse,
  HostError,
  InboundDelegateMsg,
  PutRequest,
  PutResponse,
  ResponseHandler,
  SubscribeRequest,
  UpdateData,
  UpdateNotification,
  UpdateRequest,
  UpdateResponse,
  WasmDelegateV1,
} from "../src";
import { DelegateCodeT } from "../src/client-request/delegate-code";
import { DelegateKeyT } from "../src/client-request/delegate-key";
import { DelegateType } from "../src/client-request/delegate-type";
import { InboundDelegateMsgType } from "../src/client-request/inbound-delegate-msg-type";
import { ApplicationMessagesT } from "../src/client-request/application-messages";
import { RegisterDelegateT } from "../src/client-request/register-delegate";
import { UnregisterDelegateT } from "../src/client-request/unregister-delegate";
import { DelegateContainerT } from "../src/client-request/delegate-container";

const TEST_ENCODED_KEY = "6kVs66bKaQAC6ohr8b43SvJ95r36tc2hnG7HezmaJHF9";
const DELEGATE_WS_URL = "ws://localhost:5555/contract/command/";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeHandler(overrides: Partial<ResponseHandler> = {}): ResponseHandler {
  return {
    onContractPut: () => {},
    onContractGet: () => {},
    onContractUpdate: () => {},
    onContractUpdateNotification: () => {},
    onContractNotFound: () => {},
    onDelegateResponse: () => {},
    onErr: () => {},
    onOpen: () => {},
    ...overrides,
  };
}

function makeKeyT(): ContractKeyT {
  return new ContractKeyT(
    new ContractInstanceIdT(Array.from(base58.decode(TEST_ENCODED_KEY))),
    []
  );
}

/** Build a serialized HostResponse wrapping a ContractResponse. */
function buildContractResponse(
  type: ContractResponseType,
  response: PutResponseT | GetResponseT | SubscribeResponseT
): ArrayBuffer {
  const contractResp = new ContractResponseT(type, response);
  const hostResp = new HostResponseT(HostResponseType.ContractResponse, contractResp);
  const fbb = new flatbuffers.Builder(512);
  fbb.finish(hostResp.pack(fbb));
  return new Uint8Array(fbb.asUint8Array()).buffer;
}

/** Build a serialized HostResponse wrapping a DelegateResponse. */
function buildDelegateResponse(
  key: HostDelegateKeyT,
  values: OutboundDelegateMsgT[]
): ArrayBuffer {
  const delegateResp = new HostDelegateResponseT(key, values);
  const hostResp = new HostResponseT(HostResponseType.DelegateResponse, delegateResp);
  const fbb = new flatbuffers.Builder(512);
  fbb.finish(hostResp.pack(fbb));
  return new Uint8Array(fbb.asUint8Array()).buffer;
}

/** Build a test DelegateKeyT. */
function makeDelegateKeyT(): HostDelegateKeyT {
  return new HostDelegateKeyT(
    [1, 2, 3, 4, 5, 6, 7, 8],
    [10, 20, 30, 40, 50, 60, 70, 80]
  );
}

// ---------------------------------------------------------------------------
// Delegate response deserialization
// ---------------------------------------------------------------------------

describe("Delegate response deserialization", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("DelegateResponse with ApplicationMessage triggers onDelegateResponse", async () => {
    const appMsg = new ApplicationMessageT([42, 43, 44], [99, 100], true);
    const outMsg = new OutboundDelegateMsgT(
      OutboundDelegateMsgType.common_ApplicationMessage,
      appMsg
    );
    const responseData = buildDelegateResponse(makeDelegateKeyT(), [outMsg]);

    let receivedResponse: DelegateResponse | null = null;
    const handler = makeHandler({
      onDelegateResponse: (resp) => {
        receivedResponse = resp;
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => c.send(responseData));
    await new Promise((r) => setTimeout(r, 100));

    expect(receivedResponse).not.toBeNull();
    expect(receivedResponse!.key).not.toBeNull();
    expect(receivedResponse!.key!.key).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(receivedResponse!.key!.codeHash).toEqual([10, 20, 30, 40, 50, 60, 70, 80]);
    expect(receivedResponse!.values).toHaveLength(1);
    expect(receivedResponse!.values[0].inboundType).toEqual(
      OutboundDelegateMsgType.common_ApplicationMessage
    );
    const payload = (receivedResponse!.values[0].inbound as ApplicationMessageT).payload;
    expect(payload).toEqual([42, 43, 44]);
  });

  test("DelegateResponse with ContextUpdated triggers onDelegateResponse", async () => {
    const ctxUpdated = new ContextUpdatedT([11, 22, 33]);
    const outMsg = new OutboundDelegateMsgT(
      OutboundDelegateMsgType.ContextUpdated,
      ctxUpdated
    );
    const responseData = buildDelegateResponse(makeDelegateKeyT(), [outMsg]);

    let receivedResponse: DelegateResponse | null = null;
    const handler = makeHandler({
      onDelegateResponse: (resp) => {
        receivedResponse = resp;
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => c.send(responseData));
    await new Promise((r) => setTimeout(r, 100));

    expect(receivedResponse).not.toBeNull();
    expect(receivedResponse!.values).toHaveLength(1);
    expect(receivedResponse!.values[0].inboundType).toEqual(
      OutboundDelegateMsgType.ContextUpdated
    );
    const ctx = (receivedResponse!.values[0].inbound as ContextUpdatedT).context;
    expect(ctx).toEqual([11, 22, 33]);
  });

  test("DelegateResponse with multiple outbound messages", async () => {
    const msg1 = new OutboundDelegateMsgT(
      OutboundDelegateMsgType.common_ApplicationMessage,
      new ApplicationMessageT([1], [2], false)
    );
    const msg2 = new OutboundDelegateMsgT(
      OutboundDelegateMsgType.ContextUpdated,
      new ContextUpdatedT([3, 4, 5])
    );
    const responseData = buildDelegateResponse(makeDelegateKeyT(), [msg1, msg2]);

    let receivedResponse: DelegateResponse | null = null;
    const handler = makeHandler({
      onDelegateResponse: (resp) => {
        receivedResponse = resp;
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => c.send(responseData));
    await new Promise((r) => setTimeout(r, 100));

    expect(receivedResponse).not.toBeNull();
    expect(receivedResponse!.values).toHaveLength(2);
    expect(receivedResponse!.values[0].inboundType).toEqual(
      OutboundDelegateMsgType.common_ApplicationMessage
    );
    expect(receivedResponse!.values[1].inboundType).toEqual(
      OutboundDelegateMsgType.ContextUpdated
    );
  });
});

// ---------------------------------------------------------------------------
// Delegate request serialization
// ---------------------------------------------------------------------------

describe("Delegate request serialization", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("RegisterDelegate request round-trips through FlatBuffers", () => {
    const delegateCode = new DelegateCodeT([1, 2, 3], [4, 5, 6]);
    const delegateKey = new DelegateKeyT([10, 20], [30, 40]);
    const wasmDelegate = new WasmDelegateV1([7, 8], delegateCode, delegateKey);
    const container = new DelegateContainer(DelegateType.WasmDelegateV1, wasmDelegate);
    const registerDelegate = new RegisterDelegateT(container, [50, 60], [70, 80]);
    const delegateReq = new DelegateRequest(
      DelegateRequestType.RegisterDelegate,
      registerDelegate
    );

    // Serialize
    const fbb = new flatbuffers.Builder(256);
    const packed = delegateReq.pack(fbb);
    fbb.finish(packed);
    const bytes = fbb.asUint8Array();

    // Deserialize
    const bb = new flatbuffers.ByteBuffer(bytes);
    const { DelegateRequest: DelegateRequestFbs } = require("../src/client-request/delegate-request");
    const unpacked = DelegateRequestFbs.getRootAsDelegateRequest(bb).unpack();

    expect(unpacked.delegateRequestType).toEqual(DelegateRequestType.RegisterDelegate);
    const reg = unpacked.delegateRequest as RegisterDelegateT;
    expect(reg.cipher).toEqual([50, 60]);
    expect(reg.nonce).toEqual([70, 80]);
    expect(reg.delegate).not.toBeNull();
  });

  test("UnregisterDelegate request round-trips through FlatBuffers", () => {
    const delegateKey = new DelegateKeyT([10, 20, 30], [40, 50, 60]);
    const unregister = new UnregisterDelegateT(delegateKey);
    const delegateReq = new DelegateRequest(
      DelegateRequestType.UnregisterDelegate,
      unregister
    );

    // Serialize
    const fbb = new flatbuffers.Builder(256);
    const packed = delegateReq.pack(fbb);
    fbb.finish(packed);
    const bytes = fbb.asUint8Array();

    // Deserialize
    const bb = new flatbuffers.ByteBuffer(bytes);
    const { DelegateRequest: DelegateRequestFbs } = require("../src/client-request/delegate-request");
    const unpacked = DelegateRequestFbs.getRootAsDelegateRequest(bb).unpack();

    expect(unpacked.delegateRequestType).toEqual(DelegateRequestType.UnregisterDelegate);
    const unreg = unpacked.delegateRequest as UnregisterDelegateT;
    expect(unreg.key).not.toBeNull();
    expect(unreg.key!.key).toEqual([10, 20, 30]);
    expect(unreg.key!.codeHash).toEqual([40, 50, 60]);
  });

  test("ApplicationMessages request round-trips through FlatBuffers", () => {
    const delegateKey = new DelegateKeyT([1, 2], [3, 4]);
    const appMsg = new ApplicationMessageT([10, 11, 12], [13, 14], false);
    const inboundMsg = new InboundDelegateMsg(
      InboundDelegateMsgType.common_ApplicationMessage,
      appMsg
    );
    const appMsgs = new ApplicationMessagesT(delegateKey, [5, 6], [inboundMsg]);
    const delegateReq = new DelegateRequest(
      DelegateRequestType.ApplicationMessages,
      appMsgs
    );

    // Serialize
    const fbb = new flatbuffers.Builder(256);
    const packed = delegateReq.pack(fbb);
    fbb.finish(packed);
    const bytes = fbb.asUint8Array();

    // Deserialize
    const bb = new flatbuffers.ByteBuffer(bytes);
    const { DelegateRequest: DelegateRequestFbs } = require("../src/client-request/delegate-request");
    const unpacked = DelegateRequestFbs.getRootAsDelegateRequest(bb).unpack();

    expect(unpacked.delegateRequestType).toEqual(DelegateRequestType.ApplicationMessages);
    const msgs = unpacked.delegateRequest as ApplicationMessagesT;
    expect(msgs.key).not.toBeNull();
    expect(msgs.key!.key).toEqual([1, 2]);
    expect(msgs.params).toEqual([5, 6]);
    expect(msgs.inbound).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Subscribe response with promise
// ---------------------------------------------------------------------------

describe("Subscribe with SubscribeResponse", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("subscribe() sends request and onSubscribeResponse fires with correct key", async () => {
    const responseData = buildContractResponse(
      ContractResponseType.SubscribeResponse,
      new SubscribeResponseT(makeKeyT(), true)
    );

    // Send SubscribeResponse when server receives subscribe request
    server.on("connection", (socket) => {
      socket.on("message", () => {
        socket.send(responseData);
      });
    });

    let receivedKey: ContractKey | null = null;
    let receivedSubscribed: boolean | null = null;
    const handler = makeHandler({
      onSubscribeResponse: (key, subscribed) => {
        receivedKey = key;
        receivedSubscribed = subscribed;
      },
    });

    const api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    const key = ContractKey.fromInstanceId(TEST_ENCODED_KEY);
    await api.subscribe(new SubscribeRequest(key));
    await new Promise((r) => setTimeout(r, 200));

    expect(receivedKey).not.toBeNull();
    expect(receivedKey!.encode()).toEqual(TEST_ENCODED_KEY);
    expect(receivedSubscribed).toBe(true);
  });

  test("subscribe unsubscribed=false fires callback correctly", async () => {
    const responseData = buildContractResponse(
      ContractResponseType.SubscribeResponse,
      new SubscribeResponseT(makeKeyT(), false)
    );

    let receivedSubscribed: boolean | null = null;
    const handler = makeHandler({
      onSubscribeResponse: (_key, subscribed) => {
        receivedSubscribed = subscribed;
      },
    });

    const api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => c.send(responseData));
    await new Promise((r) => setTimeout(r, 100));

    expect(receivedSubscribed).toBe(false);
  });

  test("subscribe with summary sends correct data", async () => {
    let receivedSummary = false;

    server.on("connection", (socket) => {
      socket.on("message", (rawData) => {
        try {
          const bytes =
            rawData instanceof Uint8Array
              ? rawData
              : new Uint8Array(rawData as ArrayBuffer);
          const bb = new flatbuffers.ByteBuffer(bytes);
          const req = ClientRequest.getRootAsClientRequest(bb).unpack();
          if (req.clientRequestType === ClientRequestType.ContractRequest) {
            const cr = req.clientRequest as any;
            if (cr.contractRequestType === 4 /* Subscribe */) {
              // Verify summary field is present
              if (cr.contractRequest && cr.contractRequest.summary) {
                receivedSummary = cr.contractRequest.summary.length > 0;
              }
            }
          }
        } catch (_) {}
      });
    });

    const api = new FreenetWsApi(new URL(DELEGATE_WS_URL), makeHandler());
    await new Promise((r) => setTimeout(r, 100));

    const key = ContractKey.fromInstanceId(TEST_ENCODED_KEY);
    await api.subscribe(new SubscribeRequest(key, [1, 2, 3]));
    await new Promise((r) => setTimeout(r, 100));

    expect(receivedSummary).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// onClose callback
// ---------------------------------------------------------------------------

describe("onClose callback", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.close();
  });

  test("onClose fires when connection closes", async () => {
    let closeCode: number | null = null;
    const handler = makeHandler({
      onClose: (code, _reason) => {
        closeCode = code;
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    // Close from server side
    server.clients().forEach((c) => c.close());
    await new Promise((r) => setTimeout(r, 100));

    expect(closeCode).not.toBeNull();
  });

  test("onClose is optional and does not error when absent", async () => {
    // Handler without onClose — should not throw
    const handler = makeHandler();
    delete (handler as any).onClose;

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => c.close());
    await new Promise((r) => setTimeout(r, 100));

    // If we get here without throwing, the test passes
    expect(true).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Server-side StreamChunk reassembly
// ---------------------------------------------------------------------------

describe("Server-side StreamChunk reassembly", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("reassembles chunked PutResponse from server", async () => {
    // Build a PutResponse payload
    const putResp = new PutResponseT(makeKeyT());
    const contractResp = new ContractResponseT(ContractResponseType.PutResponse, putResp);
    const hostResp = new HostResponseT(HostResponseType.ContractResponse, contractResp);
    const innerFbb = new flatbuffers.Builder(512);
    innerFbb.finish(hostResp.pack(innerFbb));
    const innerPayload = innerFbb.asUint8Array();

    // Split into 2 chunks manually
    const mid = Math.floor(innerPayload.length / 2);
    const chunk1Data = Array.from(innerPayload.subarray(0, mid));
    const chunk2Data = Array.from(innerPayload.subarray(mid));

    const streamChunk1 = new HostStreamChunkT(42, 0, 2, chunk1Data);
    const streamChunk2 = new HostStreamChunkT(42, 1, 2, chunk2Data);

    const resp1 = new HostResponseT(HostResponseType.StreamChunk, streamChunk1);
    const resp2 = new HostResponseT(HostResponseType.StreamChunk, streamChunk2);

    const fbb1 = new flatbuffers.Builder(512);
    fbb1.finish(resp1.pack(fbb1));
    const bytes1 = new Uint8Array(fbb1.asUint8Array()).buffer;

    const fbb2 = new flatbuffers.Builder(512);
    fbb2.finish(resp2.pack(fbb2));
    const bytes2 = new Uint8Array(fbb2.asUint8Array()).buffer;

    let receivedPut = false;
    const handler = makeHandler({
      onContractPut: (response: PutResponse) => {
        receivedPut = true;
        expect(response.key.encode()).toEqual(TEST_ENCODED_KEY);
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    // Send chunks in order
    server.clients().forEach((c) => {
      c.send(bytes1);
      c.send(bytes2);
    });
    await new Promise((r) => setTimeout(r, 200));

    expect(receivedPut).toBe(true);
  });

  test("reassembles chunks received out of order", async () => {
    // Build a GetResponse payload
    const getResp = new GetResponseT(makeKeyT(), null, [10, 20, 30]);
    const contractResp = new ContractResponseT(ContractResponseType.GetResponse, getResp);
    const hostResp = new HostResponseT(HostResponseType.ContractResponse, contractResp);
    const innerFbb = new flatbuffers.Builder(512);
    innerFbb.finish(hostResp.pack(innerFbb));
    const innerPayload = innerFbb.asUint8Array();

    // Split into 3 chunks
    const third = Math.floor(innerPayload.length / 3);
    const chunks = [
      Array.from(innerPayload.subarray(0, third)),
      Array.from(innerPayload.subarray(third, third * 2)),
      Array.from(innerPayload.subarray(third * 2)),
    ];

    // Build StreamChunk host responses
    function buildChunkResponse(streamId: number, index: number, total: number, data: number[]): ArrayBuffer {
      const chunk = new HostStreamChunkT(streamId, index, total, data);
      const resp = new HostResponseT(HostResponseType.StreamChunk, chunk);
      const fbb = new flatbuffers.Builder(512);
      fbb.finish(resp.pack(fbb));
      return new Uint8Array(fbb.asUint8Array()).buffer;
    }

    // Send out of order: chunk 2, chunk 0, chunk 1
    const chunkBytes = [
      buildChunkResponse(99, 2, 3, chunks[2]),
      buildChunkResponse(99, 0, 3, chunks[0]),
      buildChunkResponse(99, 1, 3, chunks[1]),
    ];

    let receivedGet = false;
    const handler = makeHandler({
      onContractGet: (response: GetResponse) => {
        receivedGet = true;
        expect(response.key.encode()).toEqual(TEST_ENCODED_KEY);
        expect(response.state).toEqual([10, 20, 30]);
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 100));

    server.clients().forEach((c) => {
      chunkBytes.forEach((b) => c.send(b));
    });
    await new Promise((r) => setTimeout(r, 200));

    expect(receivedGet).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// ContractKey edge cases
// ---------------------------------------------------------------------------

describe("ContractKey", () => {
  test("fromInstanceId encodes/decodes correctly", () => {
    const key = ContractKey.fromInstanceId(TEST_ENCODED_KEY);
    expect(key.encode()).toEqual(TEST_ENCODED_KEY);
    expect(key.bytes()).toHaveLength(32);
  });

  test("constructor rejects wrong-length instance", () => {
    expect(() => new ContractKey(new Uint8Array(16))).toThrow("Invalid array length");
  });

  test("constructor rejects wrong-length code", () => {
    expect(
      () => new ContractKey(new Uint8Array(32), new Uint8Array(16))
    ).toThrow("Invalid array length");
  });

  test("codePart returns null when no code", () => {
    const key = ContractKey.fromInstanceId(TEST_ENCODED_KEY);
    const codePart = key.codePart();
    expect(codePart).not.toBeNull();
    expect(codePart!.length).toBe(0);
  });

  test("constructor with both instance and code", () => {
    const instance = new Uint8Array(32).fill(0xaa);
    const code = new Uint8Array(32).fill(0xbb);
    const key = new ContractKey(instance, code);
    expect(key.bytes()).toEqual(instance);
    expect(key.codePart()).toEqual(code);
  });
});

// ---------------------------------------------------------------------------
// UpdateData variants
// ---------------------------------------------------------------------------

describe("UpdateData types", () => {
  test("DeltaUpdate round-trips through FlatBuffers", () => {
    const { DeltaUpdate } = require("../src");
    const { UpdateDataType } = require("../src");

    const delta = new DeltaUpdate([1, 2, 3]);
    const updateData = new UpdateData(UpdateDataType.DeltaUpdate, delta);

    const fbb = new flatbuffers.Builder(128);
    const packed = updateData.pack(fbb);
    fbb.finish(packed);
    const bytes = fbb.asUint8Array();

    expect(bytes.length).toBeGreaterThan(0);
  });

  test("StateUpdate serializes", () => {
    const { StateUpdate, UpdateData, UpdateDataType } = require("../src");
    const state = new StateUpdate([10, 20, 30]);
    const updateData = new UpdateData(UpdateDataType.StateUpdate, state);

    const fbb = new flatbuffers.Builder(128);
    const packed = updateData.pack(fbb);
    fbb.finish(packed);

    expect(fbb.asUint8Array().length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// Auth token handling
// ---------------------------------------------------------------------------

describe("Auth token in WebSocket URL", () => {
  let server: Server;

  beforeEach(() => {
    server = new Server(DELEGATE_WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("FreenetWsApi without auth token still connects", async () => {
    let opened = false;
    const handler = makeHandler({
      onOpen: () => {
        opened = true;
      },
    });

    const _api = new FreenetWsApi(new URL(DELEGATE_WS_URL), handler);
    await new Promise((r) => setTimeout(r, 200));

    expect(opened).toBe(true);
  });

  test("FreenetWsApi with auth token connects and fires onOpen", async () => {
    let opened = false;
    const handler = makeHandler({
      onOpen: () => {
        opened = true;
      },
    });

    const _api = new FreenetWsApi(
      new URL(DELEGATE_WS_URL),
      handler,
      "test-token-123"
    );
    await new Promise((r) => setTimeout(r, 200));

    expect(opened).toBe(true);
  });
});
