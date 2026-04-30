# Changelog

## [Unreleased]

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
- This is a bincode wire-format break for `NodeDiagnosticsResponse`.
  Older clients (built against 0.6.x) that decode the
  `HostResponse::QueryResponse(QueryResponse::NodeDiagnostics(_))`
  variant will fail to deserialize the new payload. The only known
  in-tree consumer is `fdev`'s diagnostics command, which is shipped
  with `freenet-core` and rebuilt in lockstep. `freenet service report`
  was already broken on the old shape and works with the new shape
  via the local fix in freenet/freenet-core#3989.
- Added a `serde_json` round-trip regression test for
  `NodeDiagnosticsResponse` to prevent the same class of bug from
  reappearing — any future field whose key type does not serialize as
  a string would break this test at the source.

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