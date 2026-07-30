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
/// Implementations must ensure that state delta updates are *commutative*. In other words,
/// when applying multiple delta updates to a state, the order in which these updates are
/// applied should not affect the final state. Once all deltas are applied, the resulting
/// state should be the same, regardless of the order in which the deltas were applied.
///
/// Implementations must also keep the delta negligible when the requesting peer's summary
/// shows it already holds this state: the delta must not contain that state, or approach
/// its size. See [`Self::get_state_delta`].
///
/// Noncompliant behavior, such as failing to obey the commutativity rule or returning a
/// state-sized delta to a peer that is already up to date, may result in the contract
/// being deprioritized or removed from the p2p network.
pub trait ContractInterface {
    /// Verify that the state is valid, given the parameters.
    fn validate_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>;

    /// Update the state to account for the new data
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
    fn summarize_state(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError>;

    /// Generate a state delta using a summary from the current state.
    /// This along with [`Self::summarize_state`] allows flexible and efficient
    /// state synchronization between peers.
    ///
    /// # The delta to an up-to-date peer must be negligible
    ///
    /// When `summary` shows that the requesting peer already holds everything this state
    /// has, the delta carries no information, and its size has to reflect that.
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
