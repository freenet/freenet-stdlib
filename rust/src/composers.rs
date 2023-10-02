use std::error::Error;

use crate::contract_interface::{ContractInstanceId, StateDelta, StateSummary};
use crate::parameters::Parameters;

pub trait Encoding
where
    Self: Sized,
{
    fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        unimplemented!()
    }

    fn into_bytes(self) -> Result<Vec<u8>, Box<dyn Error>> {
        unimplemented!()
    }
}

pub trait ComposableParameters: Encoding {
    fn contract_id(&self) -> ContractInstanceId {
        unimplemented!()
    }
}

pub trait Syncable: Encoding + std::any::Any {
    // type Context: Syncable;
    type Parameters: ComposableParameters;
    type Summary: Encoding;
    type Delta: Encoding;

    /// corresponds to ContractInterface `validate_state`
    fn verify(
        &self,
        parameters: &Self::Parameters,
        // context: &Self::Context,
        related: &RelatedContractsContainer,
    ) -> Result<(), Box<dyn Error>> {
        unimplemented!()
    }

    /// corresponds to ContractInterface `validate_delta`
    fn verify_delta(
        &self,
        parameters: &Self::Parameters,
        delta: &Self::Delta,
    ) -> Result<(), Box<dyn Error>> {
        unimplemented!()
    }

    /// corresponds to ContractInterface `update_state`.
    fn merge(
        &mut self,
        parameters: &Self::Parameters,
        // context: &Self::Context,
        delta: &Self::Delta,
        related: &RelatedContractsContainer,
    ) -> MergeResult {
        unimplemented!()
    }

    /// corresponds to ContractInterface `summarize`
    fn summarize(&self, params: &Self::Parameters) -> Result<Self::Summary, Box<dyn Error>> {
        unimplemented!()
    }

    /// corresponds to ContractInterface `delta`
    fn delta(
        &self,
        params: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, Box<dyn Error>> {
        unimplemented!()
    }
}

// todo: including the parameters here doesn't serve any pourpouse
pub enum Related<C: Syncable> {
    /// The state was previously requested and found
    Found { parameters: C::Parameters, state: C },
    /// The state was previously requested but not found
    NotFound { parameters: C::Parameters },
    /// The state was previously requested but request is still in flight
    RequestPending { parameters: C::Parameters },
    /// The state was not previously requested, this enum can be included
    /// in the MergeResult return value which will request it
    NotRequested { parameters: C::Parameters },
}

pub enum MergeResult {
    Success,
    RequestRelated(RelatedContractsContainer),
    Error(Box<dyn Error>),
}

#[derive(Default)]
pub struct RelatedContractsContainer {}

impl RelatedContractsContainer {
    pub fn get<C: Syncable>(&self, id: &ContractInstanceId) -> Related<C> {
        todo!()
    }

    pub fn request<C: Syncable>(&mut self, request: ContractInstanceId) {
        todo!()
    }

    pub fn merge(&mut self, other: Self) {
        todo!()
    }
}

mod example {
    use super::*;

    struct ContractA {
        contract_b_0: ContractB,
        contract_b_1: ContractB,
    }
    impl Encoding for ContractA {}

    struct ContractAParams {
        contract_b_0_params: ContractBParams,
        contract_b_1_params: ContractBParams,
    }
    impl Encoding for ContractAParams {}
    impl ComposableParameters for ContractAParams {}

    struct ContractASummary;
    impl Encoding for ContractASummary {}

    struct ContractADelta {
        contract_b_0: ContractBDelta,
        contract_b_1: ContractBDelta,
    }
    impl Encoding for ContractADelta {}

    // todo: this would be derived
    impl Syncable for ContractA {
        // type Context = Self;
        type Parameters = ContractAParams;
        type Summary = ContractASummary;
        type Delta = ContractADelta;

        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            delta: &Self::Delta,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            {
                let contract_b_0_id = parameters.contract_b_0_params.contract_id();
                let Related::Found {
                    state: mut contract_b,
                    ..
                } = related.get::<ContractB>(&contract_b_0_id)
                else {
                    let mut req = RelatedContractsContainer::default();
                    req.request::<ContractB>(contract_b_0_id);
                    return MergeResult::RequestRelated(req);
                };
                contract_b.merge(
                    &parameters.contract_b_0_params,
                    &delta.contract_b_0,
                    related,
                );
            }
            {
                let contract_b_1_id = parameters.contract_b_0_params.contract_id();
                let Related::Found {
                    state: mut contract_b,
                    ..
                } = related.get::<ContractB>(&contract_b_1_id)
                else {
                    let mut req = RelatedContractsContainer::default();
                    req.request::<ContractB>(contract_b_1_id);
                    return MergeResult::RequestRelated(req);
                };
                contract_b.merge(
                    &parameters.contract_b_1_params,
                    &delta.contract_b_1,
                    related,
                );
            }
            MergeResult::Success
        }

        fn verify(
            &self,
            parameters: &Self::Parameters,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn Error>> {
            {
                self.contract_b_0
                    .verify(&parameters.contract_b_0_params, related)?;
            }
            {
                self.contract_b_1
                    .verify(&parameters.contract_b_1_params, related)?;
            }
            Ok(())
        }
    }

    struct ContractB {}
    impl Encoding for ContractB {}

    struct ContractBParams;
    impl Encoding for ContractBParams {}
    impl ComposableParameters for ContractBParams {}

    struct ContractBSummary;
    impl Encoding for ContractBSummary {}

    struct ContractBDelta;
    impl Encoding for ContractBDelta {}

    impl Syncable for ContractB {
        // type Context = ContractA;
        type Parameters = ContractBParams;
        type Summary = ContractBSummary;
        type Delta = ContractBDelta;
    }
}

mod default_impls {
    use super::*;
    use crate::contract_interface::State;

    impl Syncable for State<'static> {
        type Parameters = Parameters<'static>;
        type Summary = StateSummary<'static>;
        type Delta = StateDelta<'static>;
    }

    impl Encoding for State<'static> {
        fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
            Ok(State::from(bytes).into_owned())
        }

        fn into_bytes(self) -> Result<Vec<u8>, Box<dyn Error>> {
            Ok(self.into_bytes())
        }
    }
    impl<'a> Encoding for Parameters<'a> {}
    impl<'a> ComposableParameters for Parameters<'a> {}
    impl Encoding for StateSummary<'static> {}
    impl Encoding for StateDelta<'static> {}
}
