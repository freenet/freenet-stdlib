//! `#[contract(update_validates)]` exports `__frnt__update_validates`.
// `#[contract]` emits `cfg(feature = "freenet-main-contract")` exports, so this
// test only has something to check when that feature is enabled.
#![cfg(feature = "freenet-main-contract")]
use freenet_stdlib::prelude::*;

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

#[test]
fn opting_in_exports_the_marker() {
    assert_eq!(__frnt__update_validates(), 1);
}
