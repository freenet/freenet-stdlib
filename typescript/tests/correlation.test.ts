/**
 * Request/response correlation tests.
 *
 * The node does not guarantee that responses arrive in request order: each
 * contract operation is driven by its own task and publishes its result to the
 * per-client channel whenever it finishes (freenet-core
 * `operations/get/op_ctx_task.rs` `start_client_get` spawns a detached task;
 * results reach the socket writer through `result_router_tx` in completion
 * order). A slow GET issued first therefore resolves after a cached GET issued
 * second.
 *
 * These tests deliver responses out of order deliberately, so they pin the
 * client-side contract regardless of what the node happens to do today.
 */
import * as flatbuffers from "flatbuffers";
import base58 from "bs58";
import { Server } from "mock-socket";
import {
  ContractResponseType,
  ContractResponseT,
  HostResponseType,
  HostResponseT,
  PutResponseT,
  GetResponseT,
  UpdateResponseT,
  NotFoundT,
  SubscribeResponseT,
  ErrorT,
} from "../src/host-response";
import {
  ContractContainer,
  ContractKey,
  DelegateResponse,
  GetRequest,
  GetResponse,
  HostError,
  FreenetWsApi,
  PutRequest,
  ResponseHandler,
  StateUpdate,
  SubscribeRequest,
  UpdateData,
  UpdateRequest,
  WasmContractV1,
} from "../src";
import { ContractType } from "../src/common/contract-type";
import { ContractCodeT } from "../src/common/contract-code";
import { ContractKeyT } from "../src/common/contract-key";
import { ContractInstanceIdT } from "../src/common/contract-instance-id";
import { RelatedContractsT } from "../src/client-request/related-contracts";
import { UpdateDataType } from "../src/common/update-data-type";

const WS_URL = "ws://localhost:1240/contract/command/";

/** Distinct, deterministic contract instance ids. */
const KEY_A = new Uint8Array(32).fill(0xa1);
const KEY_B = new Uint8Array(32).fill(0xb2);
const KEY_C = new Uint8Array(32).fill(0xc3);
const ENCODED_A = base58.encode(KEY_A);
const ENCODED_B = base58.encode(KEY_B);

function keyT(instance: Uint8Array): ContractKeyT {
  return new ContractKeyT(new ContractInstanceIdT(Array.from(instance)), []);
}

function contractResponse(
  type: ContractResponseType,
  response:
    | PutResponseT
    | GetResponseT
    | UpdateResponseT
    | NotFoundT
    | SubscribeResponseT
): ArrayBuffer {
  const contractResp = new ContractResponseT(type, response);
  const hostResp = new HostResponseT(
    HostResponseType.ContractResponse,
    contractResp
  );
  const fbb = new flatbuffers.Builder(512);
  fbb.finish(hostResp.pack(fbb));
  return new Uint8Array(fbb.asUint8Array()).buffer;
}

function errorResponse(msg: string): ArrayBuffer {
  const hostResp = new HostResponseT(HostResponseType.Error, new ErrorT(msg));
  const fbb = new flatbuffers.Builder(256);
  fbb.finish(hostResp.pack(fbb));
  return new Uint8Array(fbb.asUint8Array()).buffer;
}

function getResponseFor(instance: Uint8Array, state: number[]): ArrayBuffer {
  return contractResponse(
    ContractResponseType.GetResponse,
    new GetResponseT(keyT(instance), null, state)
  );
}

function putResponseFor(instance: Uint8Array): ArrayBuffer {
  return contractResponse(
    ContractResponseType.PutResponse,
    new PutResponseT(keyT(instance))
  );
}

function subscribeResponseFor(
  instance: Uint8Array,
  subscribed: boolean
): ArrayBuffer {
  return contractResponse(
    ContractResponseType.SubscribeResponse,
    new SubscribeResponseT(keyT(instance), subscribed)
  );
}

function makeHandler(overrides: Partial<ResponseHandler> = {}): ResponseHandler {
  return {
    onContractPut: () => {},
    onContractGet: () => {},
    onContractUpdate: () => {},
    onContractUpdateNotification: () => {},
    onContractNotFound: () => {},
    onDelegateResponse: (_r: DelegateResponse) => {},
    onErr: (_e: HostError) => {},
    onOpen: () => {},
    ...overrides,
  };
}

/**
 * Builds a put whose container carries `containerKey`, defaulting to the key
 * the host will echo back. The two differ in the mismatch test below.
 */
function putRequestFor(containerKey: Uint8Array): PutRequest {
  const key = new ContractKey(containerKey);
  const contract = new WasmContractV1(new ContractCodeT([1], [1]), [1], key);
  const container = new ContractContainer(ContractType.WasmContractV1, contract);
  return new PutRequest(container, [1, 2, 3], new RelatedContractsT([]));
}

function updateRequestFor(instance: Uint8Array): UpdateRequest {
  return new UpdateRequest(
    new ContractKey(instance),
    new UpdateData(UpdateDataType.StateUpdate, new StateUpdate([1]))
  );
}

/** Lets the websocket handshake complete before the test drives the server. */
const settle = () => new Promise((r) => setTimeout(r, 100));

/** Resolves to `"pending"` when `p` has not settled within `ms`. */
async function statusOf<T>(
  p: Promise<T>,
  ms = 150
): Promise<"pending" | T | Error> {
  return Promise.race([
    p.then(
      (v) => v as T,
      (e) => e as Error
    ),
    new Promise<"pending">((r) => setTimeout(() => r("pending"), ms)),
  ]);
}

describe("request/response correlation", () => {
  let server: Server;
  let api: FreenetWsApi;

  beforeEach(() => {
    server = new Server(WS_URL);
  });

  afterEach(() => {
    server.clients().forEach((c) => c.close());
    server.close();
  });

  /** Connects an api that never auto-responds; each test drives the server. */
  async function connect(
    handler: ResponseHandler = makeHandler()
  ): Promise<FreenetWsApi> {
    api = new FreenetWsApi(new URL(WS_URL), handler);
    await settle();
    return api;
  }

  function send(data: ArrayBuffer): void {
    server.clients().forEach((c) => c.send(data));
  }

  test("out-of-order GetResponses resolve their own request, not the oldest", async () => {
    await connect();

    const getA = api.get(new GetRequest(new ContractKey(KEY_A), false));
    const getB = api.get(new GetRequest(new ContractKey(KEY_B), false));

    // B was requested second but its driver task finishes first.
    send(getResponseFor(KEY_B, [0xbb]));
    send(getResponseFor(KEY_A, [0xaa]));

    const [a, b] = await Promise.all([getA, getB]);
    expect(a.key.encode()).toEqual(ENCODED_A);
    expect(a.state).toEqual([0xaa]);
    expect(b.key.encode()).toEqual(ENCODED_B);
    expect(b.state).toEqual([0xbb]);
  });

  test("out-of-order PutResponses resolve their own request", async () => {
    await connect();

    const putA = api.put(putRequestFor(KEY_A));
    const putB = api.put(putRequestFor(KEY_B));

    send(putResponseFor(KEY_B));
    send(putResponseFor(KEY_A));

    const [a, b] = await Promise.all([putA, putB]);
    expect(a.key.encode()).toEqual(ENCODED_A);
    expect(b.key.encode()).toEqual(ENCODED_B);
  });

  test("out-of-order UpdateResponses resolve their own request", async () => {
    await connect();

    const updA = api.update(updateRequestFor(KEY_A));
    const updB = api.update(updateRequestFor(KEY_B));

    send(
      contractResponse(
        ContractResponseType.UpdateResponse,
        new UpdateResponseT(keyT(KEY_B), [0xbb])
      )
    );
    send(
      contractResponse(
        ContractResponseType.UpdateResponse,
        new UpdateResponseT(keyT(KEY_A), [0xaa])
      )
    );

    const [a, b] = await Promise.all([updA, updB]);
    expect(a.key.encode()).toEqual(ENCODED_A);
    expect(a.summary).toEqual([0xaa]);
    expect(b.key.encode()).toEqual(ENCODED_B);
    expect(b.summary).toEqual([0xbb]);
  });

  test("NotFound rejects only the get for that contract", async () => {
    await connect();

    const getA = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const getB = api.get(new GetRequest(new ContractKey(KEY_B), false));

    send(
      contractResponse(
        ContractResponseType.NotFound,
        new NotFoundT(new ContractInstanceIdT(Array.from(KEY_B)))
      )
    );

    await expect(getB).rejects.toThrow("Contract not found");
    expect(await statusOf(getA)).toEqual("pending");

    send(getResponseFor(KEY_A, [0xaa]));
    const a = (await getA) as GetResponse;
    expect(a.key.encode()).toEqual(ENCODED_A);
  });

  test("two concurrent gets for the same key both resolve", async () => {
    await connect();

    const first = api.get(new GetRequest(new ContractKey(KEY_A), false));
    const second = api.get(new GetRequest(new ContractKey(KEY_A), false));

    send(getResponseFor(KEY_A, [1]));
    send(getResponseFor(KEY_A, [2]));

    const [a, b] = await Promise.all([first, second]);
    expect(a.key.encode()).toEqual(ENCODED_A);
    expect(b.key.encode()).toEqual(ENCODED_A);
  });

  // Two concurrent requests for one contract are indistinguishable to the
  // client: the wire carries no request id, and freenet-core drops its internal
  // RequestId before `ClientEventsProxy::send`. So one answer has to settle all
  // of them. Leaving the extras queued is not a safe default — the node
  // coalesces byte-identical concurrent UPDATEs into a single transaction and
  // emits one result for both, so a per-response settle would strand the second
  // caller for the full REQUEST_TIMEOUT_MS.
  test("one GetResponse settles every pending get for that key", async () => {
    await connect();

    const first = api.get(new GetRequest(new ContractKey(KEY_A), false));
    const second = api.get(new GetRequest(new ContractKey(KEY_A), false));

    send(getResponseFor(KEY_A, [0xaa])); // one response, two waiters

    const [a, b] = await Promise.all([first, second]);
    expect(a.state).toEqual([0xaa]);
    expect(b.state).toEqual([0xaa]);
  });

  test("one error settles every pending request for that key", async () => {
    await connect();

    const first = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const second = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const other = api.get(new GetRequest(new ContractKey(KEY_B), false));

    send(errorResponse(`failed to get contract ${ENCODED_A}, reason: timeout`));

    for (const settled of [await first, await second]) {
      expect(settled).toBeInstanceOf(Error);
      expect((settled as Error).message).toMatch(/timeout/);
    }
    // Scoping still holds: the other contract is untouched.
    expect(await statusOf(other)).toEqual("pending");
  });

  test("one NotFound settles every pending get for that key", async () => {
    await connect();

    const first = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const second = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);

    send(
      contractResponse(
        ContractResponseType.NotFound,
        new NotFoundT(new ContractInstanceIdT(Array.from(KEY_A)))
      )
    );

    for (const settled of [await first, await second]) {
      expect(settled).toBeInstanceOf(Error);
      expect((settled as Error).message).toEqual("Contract not found");
    }
  });

  test("a response for an unknown key is ignored, not mis-delivered", async () => {
    let handlerCalls = 0;
    await connect(makeHandler({ onContractGet: () => (handlerCalls += 1) }));

    const getA = api.get(new GetRequest(new ContractKey(KEY_A), false));

    // A stray response for a contract nothing is waiting on.
    send(getResponseFor(KEY_C, [0xcc]));
    expect(await statusOf(getA)).toEqual("pending");

    send(getResponseFor(KEY_A, [0xaa]));
    const a = await getA;
    expect(a.key.encode()).toEqual(ENCODED_A);
    // The legacy callback API still sees both responses.
    expect(handlerCalls).toBe(2);
  });

  test("a request with no usable key fails at send, never reaching the queue", async () => {
    await connect();

    // Pins the assumption `takeMatching` relies on for having no fall-back tier
    // for uncorrelatable requests: `key` is `(required)` on Get, Update,
    // Subscribe and WasmContractV1, so flatbuffers refuses to pack a request
    // without one and `sendRequest` throws before anything is queued. If the
    // schema ever loses `required`, this goes green-to-red and the reader knows
    // the no-fall-back assumption needs revisiting.
    const keyless = api.update(
      new UpdateRequest(
        null,
        new UpdateData(UpdateDataType.StateUpdate, new StateUpdate([1]))
      )
    );
    // Asserting the reason, not merely that something threw: a bare
    // `.rejects.toThrow()` would stay green if the call started failing for an
    // unrelated cause, and the pin would quietly stop testing the schema.
    await expect(keyless).rejects.toThrow(/FlatBuffers: field \d+ must be set/);

    // Nothing was queued, so a later response for another contract is not
    // absorbed by a stranded entry.
    const getA = api.get(new GetRequest(new ContractKey(KEY_A), false));
    send(getResponseFor(KEY_A, [0xaa]));
    expect((await getA).state).toEqual([0xaa]);
  });

  test("a lone put still resolves when the host's key differs from the container's", async () => {
    await connect();

    // The node recomputes a put's key from code + parameters
    // (`ContractKey::from_params_and_code`), so the container key the client
    // supplied is advisory. With a single put in flight the answer is
    // unambiguous and must still be delivered.
    const put = api.put(putRequestFor(KEY_A));
    send(putResponseFor(KEY_C));

    const response = await put;
    expect(response.key.encode()).toEqual(base58.encode(KEY_C));
  });

  test("subscribe() resolves only once the host confirms", async () => {
    await connect();

    const sub = api.subscribe(new SubscribeRequest(new ContractKey(KEY_A)));
    expect(await statusOf(sub)).toEqual("pending");

    send(subscribeResponseFor(KEY_A, true));
    await expect(sub).resolves.toBeUndefined();
  });

  test("subscribe() rejects when the host refuses the subscription", async () => {
    await connect();

    const sub = api.subscribe(new SubscribeRequest(new ContractKey(KEY_A)));
    send(subscribeResponseFor(KEY_A, false));
    await expect(sub).rejects.toThrow(/not subscribed/i);
  });

  test("out-of-order SubscribeResponses resolve their own request", async () => {
    await connect();

    const subA = api.subscribe(new SubscribeRequest(new ContractKey(KEY_A)));
    const subB = api
      .subscribe(new SubscribeRequest(new ContractKey(KEY_B)))
      .catch((e: Error) => e);

    // B is refused, A succeeds, and B's answer arrives first.
    send(subscribeResponseFor(KEY_B, false));
    send(subscribeResponseFor(KEY_A, true));

    await expect(subA).resolves.toBeUndefined();
    expect(await subB).toBeInstanceOf(Error);
  });

  test("an error naming one contract does not reject requests for others", async () => {
    await connect();

    const getA = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const getB = api.get(new GetRequest(new ContractKey(KEY_B), false));

    // Mirrors freenet-stdlib's `ContractError::Get` Display, which renders the
    // key as base58 of the instance id.
    send(errorResponse(`failed to get contract ${ENCODED_B}, reason: timeout`));

    const b = await getB.catch((e: Error) => e);
    expect(b).toBeInstanceOf(Error);
    expect((b as Error).message).toMatch(/timeout/);
    expect(await statusOf(getA)).toEqual("pending");
  });

  test("a subscriber-limit error rejects only that subscribe", async () => {
    await connect();

    const subA = api
      .subscribe(new SubscribeRequest(new ContractKey(KEY_A)))
      .catch((e: Error) => e);
    const getB = api.get(new GetRequest(new ContractKey(KEY_B), false));

    // freenet-core builds this error with a synthetic ContractKey whose code
    // hash is zeroed (`subscriber_limit_error` in
    // contract/executor/runtime.rs). Correlation keys on the instance id, which
    // is the real one, so the synthetic code hash does not defeat the match.
    send(
      errorResponse(
        `failed to subscribe for contract ${ENCODED_A}, reason: too many subscribers`
      )
    );

    const a = await subA;
    expect(a).toBeInstanceOf(Error);
    expect((a as Error).message).toMatch(/too many subscribers/);
    expect(await statusOf(getB)).toEqual("pending");
  });

  test("an error naming one contract fails every operation on it", async () => {
    await connect();

    // Pins the imprecision `rejectForError` documents as accepted: the host
    // says which contract failed, never which operation, so a get and an update
    // on that contract both fail. Locking it in means a future change cannot
    // quietly widen it past one contract.
    const getA = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const updA = api.update(updateRequestFor(KEY_A)).catch((e: Error) => e);
    const getB = api.get(new GetRequest(new ContractKey(KEY_B), false));

    send(errorResponse(`failed to get contract ${ENCODED_A}, reason: timeout`));

    for (const settled of [await getA, await updA]) {
      expect(settled).toBeInstanceOf(Error);
      expect((settled as Error).message).toMatch(/timeout/);
    }
    expect(await statusOf(getB)).toEqual("pending");
  });

  test("a key that is a base58 prefix of another is not mistaken for it", async () => {
    await connect();

    // Base58 renders each leading zero byte as a leading '1', so ids with a long
    // run of leading zeros give short '1'-heavy strings and one key's whole
    // encoding can be a literal prefix of another's. A raw substring test would
    // read the error below as naming SHORT and fail the wrong request.
    const short = new Uint8Array(32);
    short[31] = 1; // -> "1"*31 + "2"
    const long = new Uint8Array(32);
    long[31] = 58; // -> "1"*31 + "21"
    expect(base58.encode(long).startsWith(base58.encode(short))).toBe(true);

    // Queued first, so a substring match would find it before the real target.
    const getShort = api.get(new GetRequest(new ContractKey(short), false));
    const getLong = api
      .get(new GetRequest(new ContractKey(long), false))
      .catch((e: Error) => e);

    send(
      errorResponse(
        `failed to get contract ${base58.encode(long)}, reason: timeout`
      )
    );

    expect(await getLong).toBeInstanceOf(Error);
    expect(await statusOf(getShort)).toEqual("pending");
  });

  test("an error naming no known contract still fails every pending request", async () => {
    await connect();

    // Settled through `.catch` rather than left bare: if this assertion ever
    // fails, an un-awaited rejection would surface inside a later test's
    // teardown and report the failure against the wrong test.
    const getA = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const getB = api
      .get(new GetRequest(new ContractKey(KEY_B), false))
      .catch((e: Error) => e);

    send(errorResponse("internal node error"));

    for (const settled of [await getA, await getB]) {
      expect(settled).toBeInstanceOf(Error);
      expect((settled as Error).message).toEqual("internal node error");
    }
  });

  test("a connection-wide error also fences the lone-put fall-back", async () => {
    await connect();

    // Same hazard as the timeout path, reached without fake timers: put A is
    // failed by an untargeted error, the caller retries as put B, and A's late
    // answer must not be handed to B.
    const putA = api.put(putRequestFor(KEY_A)).catch((e: Error) => e);
    send(errorResponse("internal node error"));
    expect(await putA).toBeInstanceOf(Error);

    const putB = api.put(putRequestFor(KEY_B));
    send(putResponseFor(KEY_A)); // A's answer, arriving after A gave up
    expect(await statusOf(putB)).toEqual("pending");

    send(putResponseFor(KEY_B));
    expect((await putB).key.encode()).toEqual(ENCODED_B);
  });

  // The two tests above answer with the same key the departed put was queued
  // under. That is the easy half. The fall-back exists precisely because a put's
  // answer can arrive under a key the client never sent — the host recomputes it
  // — so the fence has to hold when the late answer bears an unfamiliar key too.
  // A fence that compared contract keys would pass the two above and fail these.
  test("a late answer under a recomputed key does not hijack the next put", async () => {
    await connect();

    const putA = api.put(putRequestFor(KEY_A)).catch((e: Error) => e);
    // Targeted error: A departs through the exact-match branch of
    // rejectForError, which is a third departure path.
    send(errorResponse(`put error for contract ${ENCODED_A}, reason: refused`));
    expect(await putA).toBeInstanceOf(Error);

    const putB = api.put(putRequestFor(KEY_B));
    // A's real answer, under the key the host computed — never a container key,
    // so nothing the client could have recorded from A's request would match it.
    send(putResponseFor(KEY_C));
    expect(await statusOf(putB)).toEqual("pending");
  });

  // Named for the mechanism it actually drives: this reaches the fence through
  // rejectAllPending, not through the timeout. The timeout path has its own test
  // in the "request timeout" block below.
  test("a connection-wide error fences a later recomputed-key answer", async () => {
    await connect();

    const putA = api.put(putRequestFor(KEY_A)).catch((e: Error) => e);
    send(errorResponse("internal node error"));
    expect(await putA).toBeInstanceOf(Error);

    const putB = api.put(putRequestFor(KEY_B));
    send(putResponseFor(KEY_C)); // divergent key, unrelated to either container
    expect(await statusOf(putB)).toEqual("pending");
  });

  test("connection close still rejects every pending request", async () => {
    await connect();

    const getA = api
      .get(new GetRequest(new ContractKey(KEY_A), false))
      .catch((e: Error) => e);
    const sub = api
      .subscribe(new SubscribeRequest(new ContractKey(KEY_B)))
      .catch((e: Error) => e);

    server.clients().forEach((c) => c.close());

    expect(await getA).toBeInstanceOf(Error);
    expect(((await getA) as Error).message).toMatch(/Connection closed/);
    expect(await sub).toBeInstanceOf(Error);
  });
});

/**
 * Drives the REQUEST_TIMEOUT_MS path, which nothing else in the suite exercises.
 *
 * mock-socket cannot complete a handshake under fake timers (the socket stays in
 * CONNECTING), so the connection is established on real timers first and the
 * clock is only faked around the timeout itself.
 */
describe("request timeout", () => {
  const TIMEOUT_WS_URL = "ws://localhost:1241/contract/command/";
  let server: Server;

  beforeEach(() => {
    server = new Server(TIMEOUT_WS_URL);
  });

  afterEach(() => {
    jest.useRealTimers();
    server.clients().forEach((c) => c.close());
    server.close();
  });

  test("a late answer to a timed-out put does not hijack the next put", async () => {
    const api = new FreenetWsApi(new URL(TIMEOUT_WS_URL), makeHandler());
    await settle();

    jest.useFakeTimers();
    // Queued under the fake clock, so its REQUEST_TIMEOUT_MS timer is fake too.
    const putA = api.put(putRequestFor(KEY_A)).catch((e: Error) => e);
    jest.advanceTimersByTime(31_000);
    const timedOut = await putA;
    expect(timedOut).toBeInstanceOf(Error);
    expect((timedOut as Error).message).toEqual("Request timeout");

    // The caller retries. B is now the sole pending put, which is exactly the
    // condition the lone-put fall-back keys on.
    const putB = api.put(putRequestFor(KEY_B));
    jest.useRealTimers();

    // A's answer finally arrives. Without the abandoned-key fence it has no
    // exact match, the queue holds exactly one put, and B resolves with A's
    // response — the mis-delivery this whole change exists to remove.
    server.clients().forEach((c) => c.send(putResponseFor(KEY_A)));
    expect(await statusOf(putB)).toEqual("pending");

    // B still gets its own answer.
    server.clients().forEach((c) => c.send(putResponseFor(KEY_B)));
    expect((await putB).key.encode()).toEqual(ENCODED_B);
  });
});
