//! With the `freenet-main-contract` feature OFF, `#[contract(update_validates)]`
//! must not export `__frnt__update_validates` (the export is feature-gated in
//! the contract crate, like every other contract export).
#![cfg(not(feature = "freenet-main-contract"))]
use freenet_stdlib::prelude::*;

// Unused on purpose: with the feature off the macro emits nothing that refers to it.
#[allow(dead_code)]
struct C;

#[contract(update_validates)]
impl ContractInterface for C {
    fn validate_state(
        _parameters: Parameters<'static>,
        _state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        Ok(ValidateResult::Valid)
    }

    fn update_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
        _data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError> {
        Ok(UpdateModification::valid(state))
    }

    fn summarize_state(
        _parameters: Parameters<'static>,
        _state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        Ok(StateSummary::from(vec![]))
    }

    fn get_state_delta(
        _parameters: Parameters<'static>,
        _state: State<'static>,
        _summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        Ok(StateDelta::from(vec![]))
    }
}

// If the macro emitted this symbol without the feature, the two definitions
// would collide at compile/link time, so a build failure is the failure signal.
#[no_mangle]
pub extern "C" fn __frnt__update_validates() -> i32 {
    0
}

#[test]
fn the_marker_is_feature_gated() {
    assert_eq!(__frnt__update_validates(), 0);
}
