//! Contract interface trait definition.
//!
//! This module defines the `ContractInterface` trait which all contracts must implement.

use crate::parameters::Parameters;

use super::{
    ContractError, RelatedContracts, State, StateDelta, StateSummary, UpdateData,
    UpdateModification, ValidateResult,
};

/// Trait to implement for the contract building.
///
/// Contains all necessary methods to interact with the contract.
///
/// # Examples
///
/// Implementing `ContractInterface` on a type:
///
/// ```
/// # use freenet_stdlib::prelude::*;
/// struct Contract;
///
/// #[contract]
/// impl ContractInterface for Contract {
///     fn validate_state(
///         _parameters: Parameters<'static>,
///         _state: State<'static>,
///         _related: RelatedContracts
///     ) -> Result<ValidateResult, ContractError> {
///         Ok(ValidateResult::Valid)
///     }
///
///     fn update_state(
///         _parameters: Parameters<'static>,
///         state: State<'static>,
///         _data: Vec<UpdateData>,
///     ) -> Result<UpdateModification<'static>, ContractError> {
///         Ok(UpdateModification::valid(state))
///     }
///
///     fn summarize_state(
///         _parameters: Parameters<'static>,
///         _state: State<'static>,
///     ) -> Result<StateSummary<'static>, ContractError> {
///         Ok(StateSummary::from(vec![]))
///     }
///
///     fn get_state_delta(
///         _parameters: Parameters<'static>,
///         _state: State<'static>,
///         _summary: StateSummary<'static>,
///     ) -> Result<StateDelta<'static>, ContractError> {
///         Ok(StateDelta::from(vec![]))
///     }
/// }
/// ```
// ANCHOR: contractifce
/// # ContractInterface
///
/// This trait defines the core functionality for managing and updating a contract's state.
///
/// # Conformance requirements
///
/// Freenet does not have a central coordinator that decides which peer's state "wins".
/// Instead, every peer that holds a copy of a contract's state converges on the same
/// value by repeatedly merging the copies it receives from other peers, in whatever order
/// they happen to arrive. For that to actually converge, the merge operation this trait
/// implements has to obey the algebraic laws of a join-semilattice (the structure behind
/// state-based CRDTs):
///
/// - **Associative**: `merge(merge(A, B), C) == merge(A, merge(B, C))`. Peers merge
///   updates in whatever order they arrive, not in the order they were produced.
/// - **Commutative**: `merge(A, B) == merge(B, A)`. Two peers that received the same
///   updates in a different order must still end up in the same state.
/// - **Idempotent**: `merge(A, A) == A`. Delivery on Freenet is at-least-once, so the
///   same update can and will arrive more than once.
///
/// Merging is not a separate method on this trait: applying a full-state update through
/// [`Self::update_state`] *is* the merge, and the state it returns is the merge result.
/// This matters because it is easy to implement something that looks like a reasonable
/// "reject stale updates" check but is actually last-write-wins in disguise. A contract
/// that responds to an incoming state `B` by returning its own current state `A`
/// unchanged is asserting `merge(A, B) == A`. If the peer holding `B` runs the same
/// contract and does the same thing in reverse, it asserts `merge(B, A) == B`. Those two
/// results disagree, so commutativity is broken, the two peers never converge, and every
/// future sync between them keeps retrying and re-sending the same update forever. This
/// is a real failure mode that has been observed in deployed contracts, not a
/// hypothetical one (see [`Self::update_state`] and freenet/freenet-core#5153).
///
/// Because delivery is at-least-once, applying the same delta twice must leave the state
/// unchanged after the first application, and deltas that arrive in a different order
/// must still land on the same final state.
///
/// ## Canonical byte representation
///
/// Every valid logical state must have exactly one byte representation. Freenet compares
/// states for equality by comparing bytes; it has no way to know that two different
/// encodings represent the same logical value, so it will treat them as still-diverged
/// and keep trying to reconcile them. If an encoding has more than one way to represent
/// an equivalent value, the contract must canonicalize before returning state from
/// [`Self::update_state`]. In practice this usually means: sort map and set entries by
/// key, fix the order of fields and list elements that carry no meaningful order, and
/// avoid formats whose iteration order is not deterministic.
///
/// Deltas are not held to this rule. See [`Self::get_state_delta`].
///
/// Every state emitted by [`Self::update_state`] must also be a state the contract would
/// itself accept from [`Self::validate_state`]. A state the contract would reject cannot
/// be allowed to propagate to other peers in the first place.
///
/// Implementations must also keep the delta negligible when the requesting peer's summary
/// shows it already holds this state: the delta must not contain that state, or approach
/// its size. See [`Self::get_state_delta`].
///
/// The consequences are not hypothetical: a contract that breaks the merge laws leaves the
/// peers holding it permanently diverged, and a contract that ignores the delta-size rule
/// makes every reconciliation between an in-sync pair of peers re-ship the full state
/// forever. Both have been observed on the live network (freenet/freenet-core#5153) and
/// both amount to unbounded, unresolved bandwidth cost paid on every future sync.
/// Noncompliant behavior, such as failing to obey the associativity, commutativity, or
/// idempotence rules, or returning a state-sized delta to a peer that is already up to date,
/// may result in the contract being deprioritized or removed from the p2p network.
pub trait ContractInterface {
    /// Verify that the state is valid, given the parameters.
    ///
    /// Every state this returns [`ValidateResult::Valid`] for must already be in the
    /// contract's canonical byte encoding. See the "Canonical byte representation"
    /// section on [`ContractInterface`].
    fn validate_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>;

    /// Update the state to account for the new data.
    ///
    /// This is also the merge operation described in the "Conformance requirements"
    /// section on [`ContractInterface`]. Given two states `A` and `B` for the same
    /// contract, applying `B` as an update to a peer whose current state is `A` must
    /// produce the same result as applying `A` as an update to a peer whose current
    /// state is `B`, and that result must be stable under repetition and grouping:
    ///
    /// - `merge(A, A) == A`
    /// - `merge(A, B) == merge(B, A)`
    /// - `merge(merge(A, B), C) == merge(A, merge(B, C))`
    ///
    /// Returning the current state unchanged, without incorporating anything from the
    /// incoming update, is a merge result like any other and is subject to these same
    /// laws. It is only correct when the incoming update is already implied by the
    /// current state; using it to mean "ignore updates from this source" or "keep
    /// whichever state arrived first" breaks commutativity and stalls convergence
    /// between the two peers indefinitely.
    ///
    /// The same at-least-once delivery guarantee applies to deltas: applying a delta a
    /// peer has already applied must be a no-op, and deltas that arrive out of order
    /// must still converge to the same state as deltas applied in order.
    fn update_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError>;

    /// Generate a concise summary of a state that can be used to create deltas
    /// relative to this state.
    ///
    /// The summary must be much smaller than the state it summarizes. A summary whose
    /// size is comparable to the state defeats delta computation, and a summary that is
    /// a copy of the state is always a bug. See [`Self::get_state_delta`] for the
    /// delta-size requirement this summary feeds into.
    ///
    /// This must be deterministic: the same state and parameters must always produce
    /// exactly the same summary bytes. Peers compare summaries to decide whether they
    /// have converged, so a summary that varies between calls on identical input, for
    /// example because it iterates a hash map or set in an order that is not fixed,
    /// makes two peers holding identical state look permanently divergent to each other.
    /// This class of bug has occurred in practice; see freenet/freenet-core#4857.
    fn summarize_state(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError>;

    /// Generate a state delta using a summary from the current state.
    /// This along with [`Self::summarize_state`] allows flexible and efficient
    /// state synchronization between peers.
    ///
    /// This must be deterministic for identical inputs: the same state, parameters, and
    /// summary must always produce the same delta bytes. Unlike state (see "Canonical
    /// byte representation" on [`ContractInterface`]), a delta does not need a single
    /// global canonical encoding. Two peers reconciling from different starting points
    /// may legitimately produce different delta bytes that both correctly bring the
    /// receiver to the same converged state, and delta correctness is checked by
    /// applying the delta and comparing the resulting canonical states, not by comparing
    /// delta bytes directly.
    ///
    /// # The delta to an up-to-date peer must be negligible
    ///
    /// When `summary` shows that the requesting peer already holds everything this state
    /// has, the delta carries no information, and its size has to reflect that. This is
    /// an efficiency requirement, not a correctness one: an oversized delta still
    /// converges, it just wastes bandwidth doing so, and this has been observed in
    /// deployed contracts (freenet/freenet-core#5072, freenet/freenet-core#5056).
    ///
    /// - **MUST NOT** return a delta that contains the state, or whose size approaches the
    ///   state's. This is the actual defect. A `get_state_delta` that ignores `summary`
    ///   and returns the whole state makes every reconciliation re-ship data the peer
    ///   already holds, forever, and is the behavior that may get a contract
    ///   deprioritized or removed from the network.
    /// - **SHOULD** return a literally empty delta, `StateDelta::from(vec![])`. Peers read
    ///   zero bytes as an unambiguous "converged" and skip the broadcast, so this is the
    ///   cheapest and clearest answer.
    /// - **Acceptable**: a small fixed amount of encoding framing. Serializing a delta
    ///   struct whose fields are all `None` or empty costs about a byte per field with
    ///   bincode, and tens of bytes with CBOR, since ciborium writes field names. That
    ///   is not the unambiguous converged signal, so prefer zero, but it is not a defect
    ///   and carries no penalty.
    ///
    /// What matters is delta size relative to state size, not the exact byte count. Twenty
    /// bytes against a 500 KB state is fine. A state-sized delta is not.
    ///
    /// Building state with `freenet-scaffold` gets the empty case for free: its
    /// `#[composable]` derive collapses an all-`None` delta struct to `None`, which the
    /// contract maps to `StateDelta::from(vec![])`. A hand-rolled `get_state_delta` has
    /// no such collapse and must add the check itself if it wants zero bytes.
    fn get_state_delta(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError>;
}
// ANCHOR_END: contractifce
