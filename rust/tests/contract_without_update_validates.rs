//! Without the argument, `#[contract]` must not export `__frnt__update_validates`.
// `#[contract]` emits `cfg(feature = "freenet-main-contract")` exports, so this
// test only has something to check when that feature is enabled.
#![cfg(feature = "freenet-main-contract")]
use freenet_stdlib::prelude::*;

struct C;

#[contract]
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

// If the macro also emitted this symbol, the two definitions would collide at
// compile/link time, so this test failing to build is the failure signal.
#[no_mangle]
pub extern "C" fn __frnt__update_validates() -> i32 {
    0
}

#[test]
fn not_opting_in_does_not_export_the_marker() {
    assert_eq!(__frnt__update_validates(), 0);
}
