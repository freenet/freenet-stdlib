//! Includes testing for logging
// ANCHOR: contractifce
use freenet_stdlib::{info, prelude::*};

pub const RANDOM_SIGNATURE: &[u8] = &[6, 8, 2, 5, 6, 9, 9, 10];

struct Contract;

#[contract]
impl ContractInterface for Contract {
    fn validate_state(
        _parameters: Parameters<'static>,
        _state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        unimplemented!()
    }

    fn update_state(
        _parameters: Parameters<'static>,
        _state: State<'static>,
        _data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError> {
        info!("state update");
        todo!()
    }

    fn summarize_state(
        _parameters: Parameters<'static>,
        _state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        unimplemented!()
    }

    fn get_state_delta(
        _parameters: Parameters<'static>,
        _state: State<'static>,
        _summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        unimplemented!()
    }
}
// ANCHOR_END: contractifce

fn main() {}

#[test]
fn test_log() {
    use tracing::level_filters::LevelFilter;

    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(LevelFilter::INFO)
        .init();
    info!("state update");
}
