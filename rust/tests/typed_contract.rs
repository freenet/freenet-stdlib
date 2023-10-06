use freenet_macros::contract;
use freenet_stdlib::{composers::MergeResult, prelude::*, typed_contract::TypedContract};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Contract;
#[derive(Serialize, Deserialize)]
pub struct CParams;
#[derive(Serialize, Deserialize)]
pub struct CDelta;
#[derive(Serialize, Deserialize)]
pub struct CSummary;

// use freenet_stdlib::prelude::SerializationAdapter;
// impl SerializationAdapter for Contract {
//     type Parameters = CParams;
//     type Delta = CDelta;
//     type Summary = CSummary;
//     type SelfEncoder = BincodeEncoder<Self>;
//     type ParametersEncoder = BincodeEncoder<Self::Parameters>;
//     type DeltaEncoder = BincodeEncoder<Self::Delta>;
//     type SummaryEncoder = BincodeEncoder<Self::Summary>;
// }

#[contract(
    encoder = BincodeEncoder, 
    types(
        type Parameters = CParams;
        type Delta = CDelta;
        type Summary = CSummary;
    )
)]
impl TypedContract for Contract {
    fn verify(
        &self,
        _: Self::Parameters,
        _: freenet_stdlib::composers::RelatedContractsContainer,
    ) -> Result<freenet_stdlib::prelude::ValidateResult, freenet_stdlib::prelude::ContractError>
    {
        unimplemented!()
    }

    fn verify_delta(
        _: Self::Parameters,
        _: Self::Delta,
    ) -> Result<bool, freenet_stdlib::prelude::ContractError> {
        unimplemented!()
    }

    fn merge(
        &mut self,
        _: &Self::Parameters,
        _: freenet_stdlib::typed_contract::TypedUpdateData<Self>,
        _: &RelatedContractsContainer,
    ) -> freenet_stdlib::composers::MergeResult {
        MergeResult::Success
    }

    fn summarize(
        &self,
        _: Self::Parameters,
    ) -> Result<Self::Summary, freenet_stdlib::prelude::ContractError> {
        Ok(CSummary)
    }

    fn delta(
        &self,
        _: Self::Parameters,
        _: Self::Summary,
    ) -> Result<Self::Delta, freenet_stdlib::prelude::ContractError> {
        unimplemented!()
    }
}
