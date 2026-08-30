# Changelog

## [Unreleased]

### Added

- **`DelegateCtx::list_subscriptions`** — a delegate can now ask the node which
  contracts it is subscribed to. Backed by two new V2 host functions in the
  `freenet_delegate_contracts` import namespace,
  `__frnt__delegate__list_subscriptions_len` and
  `__frnt__delegate__list_subscriptions`, with
  `encode_contract_id_list` / `decode_contract_id_list` as the shared codec.

  A delegate's subscription set lives in the node: the WASM is instantiated per
  invocation and dropped afterwards, and the node replays subscriptions across a
  restart without running the delegate at all. Until now a delegate had no way
  to learn its own state after a restart — it could only keep a parallel record
  in its secrets, which drifts from the node's exactly when it matters, or
  re-subscribe to everything on every wake. See freenet-core#5467.

  It returns `Result<Vec<[u8; 32]>, i64>`, not a bare `Vec`. An empty list and a
  failed enumeration mean opposite things to a caller deciding whether to
  re-subscribe, so they must not share a representation.

  **This is additive for every existing delegate.** Host functions resolve by
  name at module instantiation, so a delegate that does not import it is
  unaffected, and one that does fails to *load* on a node too old to provide it
  — a named missing-import error, rather than a silent failure mid-protocol.
  **Requires a node whose freenet-core registers these imports**; calling it
  against an older node is a load-time failure, by design.

### Fixed

- **`InboundDelegateMsg`'s doc comment claimed `OutboundDelegateMsg` was
  `#[non_exhaustive]`. It never has been.** The comment is corrected, and the
  asymmetry is now documented as the deliberate choice it is, on both enums.

  `OutboundDelegateMsg` is **staying** un-marked, and should not be "fixed" by
  marking it. Every variant is a request the host must act on, and freenet-core
  dispatches them in exhaustive matches with no wildcard (`contract.rs`, in the
  request loop and in the app-message filter). Marking the enum would force
  those to grow `_ =>` arms, and a newly added variant would then compile
  against the host with no handler — the delegate's request silently swallowed,
  the call reporting success. That is the failure mode a delegate
  `SubscribeContractRequest` has today, and the compile error is what prevents
  the next one. `InboundDelegateMsg` keeps the attribute because its consumers
  are third-party delegate WASM, which can reasonably ignore an unknown variant.

- **`DelegateCtx::subscribe_contract`'s doc comment** said notification delivery
  was "a follow-up" (it works), and said nothing about whether a delegate
  subscription registers demand in the network.

  It now says that **whether it does is a property of the node, not of this
  library, and a delegate cannot detect which it has**. On nodes predating
  freenet-core#4669 a subscribe registers no demand at all — no pin, no renewal
  set, no eviction exemption — so the delegate sees remote updates only while
  something else keeps the node subscribed, typically an open UI client. Once
  #4669 lands it registers demand when the node is hosting the contract, and
  still does not when the node can resolve but is not hosting it, since a pin on
  a contract the node does not hold could be neither renewed nor reclaimed.

  The call reports success in every one of those cases and nothing
  distinguishes them, which is why this belongs at the call site rather than in
  an issue. It is deliberately phrased as current node behaviour with a tracking
  reference rather than as a property of the API, so that freenet-core#4669
  landing makes it incomplete rather than wrong.

### Internal — wire-format guards

- **Every variant of both delegate message enums now has its bincode tag
  pinned** (`delegate_msg_variant_tags_are_pinned`). The previous pin covered
  `InboundDelegateMsg`'s variant 0 alone, so any reorder that left
  `ApplicationMessage` first went undetected — including swapping
  `UserResponse` and `GetContractResponse`, which reassigns two tags and makes
  deployed delegate WASM decode each as the other, silently. That exact swap was
  written during the work that produced this pin, which is the argument for it.

  The guard fails closed in both directions: the tag map is an exhaustive
  `match`, so a new variant is a compile error until it is pinned, and a probe
  asserts that the next tag along does not decode, so a variant cannot be added
  without the count constants noticing.

- **The compatibility rules are now asserted, not just documented**
  (`delegate_wire_compat` in `delegate_interface.rs`, `struct_field_wire_compat`
  in `client_events.rs`). Worth reading before changing anything on the wire,
  because appending an enum variant and appending a struct field break in
  **opposite** directions:

  | change | old sender → new receiver | new sender → old receiver |
  |---|---|---|
  | append an enum variant | fine, old tags unchanged | hard error, unknown tag |
  | append a struct field | **hard error**, unexpected end of input | silently ignored *if the struct is terminal in its message*; **silent corruption** if it is not |

  So a struct field is the more dangerous of the two: bincode is positional and
  carries no field tags, so there is nothing for a decoder to skip, and
  `#[serde(default)]` does not help — it is a self-describing-format feature and
  protects the `serde_json` path only. The practical rule is to prefer a new
  enum variant over a new field on an existing wire struct, since a variant is
  only seen by a peer that asked for it.

  `NodeDiagnosticsResponse` happens to be terminal in its message, which is the
  only reason a field could be appended to it without corrupting anything after
  it. That property is now pinned rather than assumed.

### TypeScript SDK — Breaking (npm package `@freenetorg/freenet-stdlib`; next release must be 0.4.0, not a patch)

The npm package is versioned separately from the Rust crate. This release
changes runtime behavior for existing callers, so on a 0.x line it takes a
minor bump: **0.3.0 -> 0.4.0**, not 0.3.1.

- **Host responses are now correlated to requests by contract key.**
  `FreenetWsApi` previously resolved the *oldest* pending request of a type
  with whatever response arrived, with no correlation at all. Two concurrent
  `get()`s for different contracts whose answers came back out of order
  therefore resolved into each other's promises — one contract's state
  delivered to the other's caller, silently. This is reachable in practice, not
  theoretical: freenet-core drives each contract operation on its own task and
  publishes results as they complete, so responses arrive in completion order,
  not request order.

  Every `ContractResponse` carries the contract key (or, for `NotFound`, the
  instance id), so requests now record the key they expect back and are matched
  on it. A response that matches no pending request is dropped rather than
  mis-delivered; the `ResponseHandler` callbacks still see it, so the legacy
  callback API is unchanged.

  One response settles *every* pending request for its key. Concurrent requests
  for one contract are indistinguishable — the wire carries no request id — and
  the node coalesces byte-identical concurrent UPDATEs into a single
  transaction, emitting one result for both, so settling only the oldest would
  strand the rest until the 30s timeout.

- **`subscribe()` now waits for the host's confirmation.** It previously
  returned a `Promise<void>` that resolved as soon as the request was *sent*,
  and could never reject — a refused subscription (a subscriber-limit rejection,
  for instance) left the awaited promise silently resolved. It now resolves on
  `SubscribeResponse { subscribed: true }` and rejects on `subscribed: false`,
  on a host error naming the contract, on connection close, or after
  `REQUEST_TIMEOUT_MS`. **Callers who `await subscribe()` see behavior change**:
  the call now blocks until the host answers, and can throw where it previously
  could not. Fire-and-forget callers should add a rejection handler.

- **A host error no longer fails every in-flight request.** Any single error
  previously rejected every pending get, put and update across all contracts.
  Errors are now scoped to the requests whose contract key the error message
  names. An error naming no pending contract still fails everything, since it
  may be connection-wide and must not leave callers waiting out the timeout.

### Breaking (next release must be 0.9.0, not a patch)
- **`DelegateRequest::RegisterDelegateWithPredecessors`** removed (added in
  0.8.4). freenet-core's node-side handler for this request was disabled in
  freenet-core#5199 (tracking issue freenet-core#5198): its `origin_contract`
  authorization gate — meant to confirm the registering web-app actually owns
  the predecessor delegate before copying its secrets — is forgeable by any
  HTTP client, letting an attacker who knows a target app's public contract id
  register a delegate that names the target's real delegate as predecessor
  and receive its `Local`-scope secrets.

  No known client ever constructed or sent this variant on the wire — River,
  ghostkeys, and Atlas each carry their own client-driven secret/state
  continuity mechanism instead — so removing it changes no deployed runtime
  behavior. It IS still referenced by name in freenet-core's own source
  (match arms plus the #5198 regression tests, which construct the variant
  directly to prove the vulnerability is closed); those references, and what
  replaces the regression coverage, must be addressed together when
  freenet-core's `freenet-stdlib` dependency is next bumped past this
  version — tracked in freenet-core#5201. Removing a public enum variant is
  source-breaking for any external code that names or constructs it (even
  though the enum is `#[non_exhaustive]`, which only protects an exhaustive
  *match*, not a constructor), so this requires a semver-minor release
  (0.9.0), not a patch — a plain `0.8.6` would surprise anyone whose
  `Cargo.toml` pins `"0.8"` and picks it up via `cargo update`.

  It was appended as the last variant of `DelegateRequest` specifically so
  this removal doesn't reassign any other variant's bincode tag —
  `ApplicationMessages` (0), `RegisterDelegate` (1), and
  `UnregisterDelegate` (2) are unaffected.

## [0.8.5] - 2026-07-27

### Fixed
- **Related-contract decoding no longer panics on well-formed requests.**
  `RelatedStateUpdate.related_to`, `RelatedDeltaUpdate.related_to`,
  `RelatedStateAndDeltaUpdate.related_to` and `RelatedContract.instance_id`
  were decoded with `ContractInstanceId::from_bytes(..).unwrap()`. That
  function is a **base58 string decoder**, and the wire carries the id as 32
  raw bytes. This was not an edge case: a random 32-byte id essentially never
  consists solely of base58 characters (the alphabet is 58 of 256 byte values,
  so the odds are about 2e-21), so *every* FlatBuffers UPDATE carrying a
  related update, and every PUT carrying a related contract, panicked the
  client's connection task. The PUT case stayed hidden because the loop body
  only runs on a non-empty vector and the TypeScript suite's fixture passes an
  empty one.

  Stated plainly, because it is not purely a panic fix: those four fields are
  now **raw-bytes-only**. The old decoder did accept base58 *text* there (that
  is the one input base58 decoding handles), so a client that worked around the
  panic by sending text at exactly those fields is rejected now. Raw bytes is
  what the schema type carries everywhere else, including `ContractKey.instance`
  in the same request.

- **Four more length-unchecked `(required)` fields no longer panic.**
  `DelegateKey.key`, `SecretsId.hash`, `RegisterDelegate.cipher` and
  `RegisterDelegate.nonce` were read with `copy_from_slice` or
  `try_from(..).unwrap()` into fixed-size arrays. The flatbuffers verifier
  checks that a `(required)` vector is PRESENT, not that it is the right
  LENGTH, so any client could send a short one and take down its connection
  task. `DelegateKey` is on the normal delegate path, and the TypeScript SDK
  exports it as the raw generated type with no length validation.

- **Four union discriminants no longer hit `unreachable!()`.** `ContractType`,
  `DelegateType`, `UpdateDataType` and `InboundDelegateMsgType` were decoded
  with `unreachable!()` on an unrecognized discriminant, but every generated
  union verifier ends in `_ => Ok(())`, so any discriminant a client sets
  reaches the decoder's match. All four now return a per-request error,
  matching what `ContractRequestType` and `DelegateRequestType` already did;
  those two, plus `ClientRequestType`, now share the same error shape and all
  report the offending value.

  A single test sweeps all 256 discriminants of all seven unions currently on
  the decode path, and a source-scrape test fails CI if a new decoder
  reintroduces either shape.

- **`HostResponse`'s three related-update variants now encode `related_to` as
  raw bytes.** They wrote `related_to.encode()` - base58 *text* - into
  `common.ContractInstanceId.data`, which every other producer and every
  consumer treats as 32 raw bytes. This is the encode half of the same bug, and
  it survived because Rust only encodes host responses while only TypeScript
  decodes them, so no round-trip test ever crossed it.

Every item above is **FlatBuffers-only**. The native (bincode) path decodes the
same Rust enums directly and never reaches `try_decode_fbs`, and first-party
tooling uses `encodingProtocol=native` — which is why this class went unnoticed
and why it lands on third-party and browser clients.

- **`ContractKey::try_decode_fbs` no longer double-hashes the code hash.** The
  wire `code` field carries the already-computed 32-byte code hash, but the
  decoder passed it to `CodeHash::from_code`: the *hashing* constructor -
  producing `BLAKE3(BLAKE3(wasm))`, a key that never matches the store. Every
  FlatBuffers `UpdateRequest` failed as a result, which the client surfaced as
  a 30-second timeout rather than an error. GET and SUBSCRIBE were unaffected
  because their request variants carry only an instance id and never decode
  `code`. The regression dates to `844880e` (Nov 2023), which changed a
  pass-through `CodeHash::new` into `from_code`. Blast radius was FlatBuffers
  clients, the TypeScript SDK's default, so browser apps: the audience least
  able to diagnose it.

- **A wrong-length `instance` no longer panics the connection task.**
  `instance`/`data` are `(required)` in the schema, but the flatbuffers
  verifier checks that a required vector is present, not that it is the right
  length. An 8-byte instance passed verification and then hit a
  `try_into().unwrap()`, panicking with `TryFromSliceError`; nothing catches
  unwind on that path, so a malformed message from any peer killed that
  client's connection task. The `ContractKey.instance` field is now
  length-checked at all three sites that decode it (UPDATE, GET, SUBSCRIBE).

  That entry covered the `ContractKey` fields only; the rest of the same class
  elsewhere on the decode path - including `UpdateData`'s `Related*` variants -
  is fixed by the entries above, closing freenet-core#4996.

### Deprecated
- **`ContractInstanceId::from_bytes` is renamed to
  `ContractInstanceId::from_base58`.** It parses base58 *text*, not raw bytes;
  the old name and its "build from the binary representation" doc are what
  caused the four decode sites above to feed it raw wire bytes. `from_bytes`
  remains as a delegating alias, so the rename is not a breaking change, but it
  now warns. Use `ContractInstanceId::new` for raw bytes you already hold.

### Compatibility
- **A `ContractKey` whose `code` field is absent or not exactly 32 bytes is now
  rejected at decode.** This is a hard rejection of a shape `common.fbs`
  permits (`code` is not `(required)`) and that the TypeScript SDK actually
  emits: `ContractKey.fromInstanceId(...)` produces a present-but-zero-length
  vector, and the SDK's own test suite builds an `UpdateRequest` that way.

  **No working client regresses.** Such an UPDATE has never succeeded. The node
  gates an UPDATE on already holding the contract's code blob and probes for it
  by code hash; an UPDATE supplies no contract code, so a zero-length `code`
  hashed to `BLAKE3("")` and failed at that gate. What changes is *where* and
  *how* it fails: previously a 30-second timeout or an opaque
  `"missing contract: <key>"` pointing at the node, now an immediate
  `ContractKey.code must be the 32-byte contract code hash; got 0 bytes...`
  naming the field and the remedy.

  The real fix is for the node to resolve the code hash from the instance id,
  as GET and SUBSCRIBE already do via `code_hash_from_id`: tracked in
  freenet-core#4978. Until then, build keys with both parts:
  `new ContractKey(instance, code)` rather than
  `ContractKey.fromInstanceId(...)`.

## [0.8.4] - 2026-07-21

### Added
- **`DelegateRequest::RegisterDelegateWithPredecessors`** — a new client
  request variant for registering a delegate while requesting a one-shot,
  node-side copy-forward of the LOCAL-scope secrets belonging to one or more
  retired delegate generations (`predecessors: Vec<DelegateKey>`) into the new
  delegate's namespace. This is the wire primitive for consent-gated,
  one-click delegate migration (freenet-core#4117, freenet-core#2776): the
  node copies already-sealed secret bytes only, performing no execution of any
  old delegate WASM. The copy-forward is best-effort over unknown
  predecessors and idempotent per `(predecessor -> successor)` pair. `cipher`
  and `nonce` mirror `RegisterDelegate` for field-shape parity and are ignored
  by the node (as they have been since freenet-core#4140).

  The variant is **appended last** (bincode tag `3`, one past
  `UnregisterDelegate`), so the encodings of `ApplicationMessages`,
  `RegisterDelegate`, and `UnregisterDelegate` are byte-for-byte unchanged and
  older clients keep working. New wire-format pin tests freeze the complete
  bincode byte vector of all four variants (the three pre-existing ones
  anchored to the shipped `0.8.3` format) to guard against a future reorder or
  nested-encoding change. The flatbuffers (`EncodingProtocol::Flatbuffers`)
  client path is intentionally not extended; the Rust consumers of this feature
  use the bincode `Native` path.

### Fixed
- **The flatbuffers decode path no longer panics on an unknown union
  discriminant.** Both `DelegateRequest::try_decode_fbs` and
  `ContractRequest::try_decode_fbs` matched their generated union type with an
  `unreachable!()` catch-all, but it IS reachable: the generated verifier
  accepts any discriminant it doesn't recognize (`_ => Ok(())`), and the union
  type field is a raw `u8` a client can set to any value, so a crafted request
  reached the catch-all and took down the connection handler. Both now return a
  clean per-request `DeserializationError` naming the unknown discriminant.

## [0.8.3] - 2026-07-10

### Fixed
- **WASM `WebApi` leaked every inbound WebSocket message for the life of
  the tab.** `WebApi::start`'s `onmessage` handler decoded each incoming
  Blob with a per-message `FileReader` whose `onloadend` closure was
  `forget()`-leaked; the leaked closure pinned the `FileReader`, and
  `FileReader.result` pinned the full decoded payload, so every inbound
  message's bytes were retained forever. Long-lived consumers (River)
  grew to multi-GB tab memory in both Chrome and Firefox. The socket now
  sets `binaryType = "arraybuffer"` and decodes `e.data()` synchronously
  in `onmessage`, removing the Blob → `FileReader` async hop entirely, so
  there is no per-message closure to leak and no `FileReader` to pin
  payloads. Same wire format, same `HostResult` dispatch, same streaming
  reassembly path — only the frame-decode transport changed. A non-binary
  frame now reports a connection error through the normal error handler
  instead of crashing on an unchecked cast. See
  freenet/freenet-core#4746.
- **`BorrowMutError` → WASM abort on any malformed or duplicate stream
  chunk.** The `receive_chunk` `borrow_mut()` in the reassembly `match`
  scrutinee lived until the end of the `match`, so the error arm's
  `remove_stream` re-borrow panicked (`BorrowMutError`, aborting the WASM
  instance) on any malformed or duplicate stream chunk. The borrow is now
  hoisted out of the scrutinee. `binaryType` is also set before the
  handlers are installed so the ordering is self-evidently safe rather
  than relying on `start()` being synchronous. Pre-existing on `main`,
  surfaced by PR review.

## [0.8.0]

### Fixed
- `CodeHash::encode` no longer lowercases its Base58 output. The
  BITCOIN alphabet is case-sensitive, so lowercasing corrupted the
  bytes for any hash whose encoding contained uppercase characters and
  broke the `encode` → `ContractKey::from_params` roundtrip (which
  decodes with the same case-sensitive alphabet). `CodeHash::encode`
  now matches `ContractInstanceId::encode`,
  `ContractKey::encoded_code_hash`, and `ContractCode::hash_str`, all
  of which already preserved case. See freenet/freenet-core#4214.

## [0.7.0]

### Fixed (wire-format break in `NodeDiagnosticsResponse`)
- `NodeDiagnosticsResponse.contract_states` is now
  `HashMap<String, ContractState>` (Base58 contract id) instead of
  `HashMap<ContractKey, ContractState>`. The previous type had a
  derived `Serialize` for `ContractKey` that emitted a struct
  (`{instance, code}`), which `serde_json` rejects because JSON object
  keys must be strings — every diagnostic report from a node hosting at
  least one contract uploaded with empty `network_status`. The new key
  matches the convention every other field in this struct already uses
  (`peer_id: String`, `connected_peers: Vec<(String, String)>`,
  `ContractHostingEntry::contract_key: String`). See
  freenet/freenet-core#3987.

### Compatibility
- This is a **bidirectional bincode wire-format break** for
  `NodeDiagnosticsResponse`. Bincode encodes
  `HashMap<ContractKey, ContractState>` and
  `HashMap<String, ContractState>` as different byte sequences for the
  same logical data, so:
  - Older clients built against 0.6.x will fail to deserialize a
    `HostResponse::QueryResponse(QueryResponse::NodeDiagnostics(_))`
    payload produced by a 0.7-or-newer node.
  - Newer clients built against 0.7.0 will fail to deserialize the same
    variant produced by an older node.
  Every other variant in `HostResponse`/`QueryResponse` is unchanged;
  the `#[non_exhaustive]` enum discriminants from 0.6.0 still hold.
  Run matched versions across gateway and tooling.
- The Base58 stringification via `ContractKey::Display` drops the
  `code_hash` field that the broken derived serializer would have
  emitted alongside `instance`. No in-tree consumer reads `code_hash`
  from this map; future consumers that need it will need a separate
  field.
- Known affected consumer sites that need source updates when
  freenet-core bumps to a release including this stdlib:
  - `crates/core/src/node/network_bridge/p2p_protoc.rs:1900,1916`
    (producer-side: `.insert(contract_key, ...)` becomes
    `.insert(contract_key.to_string(), ...)`)
  - `crates/fdev/src/diagnostics.rs:158-170` (consumer-side: already
    iterates and calls `.to_string()` on the key — source-compatible,
    no change required)
  - `crates/core/src/bin/commands/report.rs::diagnostics_to_json`
    (the workaround merged in freenet/freenet-core#3989 — `.to_string()`
    on `String` is a no-op, so still correct, but the helper becomes
    redundant and can be simplified to a plain
    `serde_json::to_string_pretty(&diag)`)
  - `freenet-test-network/src/network.rs:1093` (out-of-tree consumer:
    map lookup keyed by `ContractKey` needs `.to_string()`)
- Added a `serde_json` round-trip regression test in stdlib for
  `NodeDiagnosticsResponse` (every field populated) to prevent the
  same class of bug from reappearing — any future struct field whose
  key type does not serialize as a string would break this test at
  the source.

## [0.6.0] - 2026-04-13

### Changed (source-level breaking, wire-compatible)
- Added `#[non_exhaustive]` to five wire-boundary enums so future variants
  can be added without a source-level break for downstream consumers that
  match exhaustively:
  - `delegate_interface::InboundDelegateMsg` (companion to the already-
    `non_exhaustive` `OutboundDelegateMsg`)
  - `contract_interface::update::UpdateData`
  - `delegate_interface::DelegateError`
  - `contract_interface::error::ContractError`
  - `versioning::APIVersion`
  Downstream `match` sites must now include a wildcard arm.

### Added
- Wire-format pin tests for `InboundDelegateMsg::ApplicationMessage` and
  `UpdateData::{State, Delta}`. These lock the bincode variant tags so that
  a refactor which reorders variants fails loudly at test time rather than
  silently corrupting in-flight messages to deployed contracts/delegates.

### Compatibility
- `#[non_exhaustive]` is a source-level change only. It does not affect
  bincode discriminants, serde `Serialize`/`Deserialize` impls, byte layout,
  or the wire format. Deployed contracts and delegates compiled against any
  previous 0.x stdlib continue to deserialize identically. This bump is
  minor-breaking (0.5.0 → 0.6.0) only because downstream Rust code that
  pattern-matches these enums exhaustively must add a wildcard arm to
  compile against 0.6.

## [0.5.0] - 2026-04-13

### Added
- `MessageOrigin::Delegate(DelegateKey)` variant so the runtime can attest the
  caller's identity for delegate-to-delegate `SendDelegateMessage` calls.
  Previously the receiver got `origin = None` and could not learn which
  delegate invoked it. (freenet/freenet-core#3860)

### Changed
- `MessageOrigin` is now `#[non_exhaustive]`. Source code matching on it must
  add a wildcard arm; this is a one-time source break, not a wire-format
  break — bincode discriminants for existing variants are unchanged, so
  deployed delegate WASM continues to deserialize `WebApp(..)` and `None`
  origins identically.

### Compatibility
- Wire format for `MessageOrigin::WebApp(..)` is byte-identical to 0.4.x.
- Deployed delegates only break if they start receiving inter-delegate calls
  carrying the new `Delegate(..)` variant, which no production delegate
  exercises today. Rebuild against 0.5.x is only required for delegates that
  will participate in delegate-to-delegate messaging.

## [0.1.14] - 2025-09-04

### Changed
- Updated `tokio-tungstenite` from 0.26.1 to 0.27.0
- Updated `rand` from 0.8 to 0.9 (dev dependency)
- Fixed `from_entropy()` to use `from_os_rng()` for rand 0.9 compatibility

### Note
- [AI-assisted debugging and comment]
- This release updates dependencies to support freenet-core dependency updates

## [0.1.9] - 2025-06-19

### Added
- NodeQuery enum with ConnectedPeers and SubscriptionInfo variants
- SubscriptionInfo struct for tracking contract subscriptions
- NetworkDebugInfo struct for network debugging information
- QueryResponse::NetworkDebug variant for debugging responses

### Note
- These APIs were present in 0.1.7 but missing from main branch
- This release combines the panic fix from 0.1.8 with the missing APIs from 0.1.7

## [0.1.8] - 2025-06-19

### Fixed
- Fixed panic in `APIVersion::from_u64()` when encountering unsupported version numbers
  - Now returns proper error instead of panicking
  - Prevents server crashes when loading contracts with invalid version data
  - Critical fix for River invitation bug where requests would hang indefinitely

### Changed
- `APIVersion::from_u64()` now returns `Result<Self, VersionError>` instead of `Self`
- Added `VersionError` enum for better error handling

## [0.1.7] - Previous release