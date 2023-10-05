#![allow(dead_code, unused)]
use std::error::Error;

mod parent {
    use std::error::Error;

    use super::children::{self, *};
    use freenet_stdlib::{composers::*, prelude::*};
    use serde::{Deserialize, Serialize};

    use freenet_stdlib::memory::wasm_interface::inner_validate_state;

    // todo: code in this mod should be derived
    mod low_level_ffi_impl {
        use super::*;
        use freenet_stdlib::prelude::ContractInterface;

        // todo: have a macro that can be snapped on top of `ComposableContract` that generates this?
        impl SerializationAdapter for ParentContract {
            type Parameters = ParentContractParams;
            type Delta = ParentContractDelta;
            type Summary = ParentContractSummary;
        }

        // todo: have an other macro that can be snapped on top of `impl SerializationAdapter`
        // that generated all of below, it can take a parameter (e.g. BincodeEncoder) to specify
        // the encoder to use
        impl BincodeEncoder for ParentContract {}
        impl BincodeEncoder for ParentContractParams {}
        impl BincodeEncoder for ParentContractDelta {}
        impl BincodeEncoder for ParentContractSummary {}
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContract {
        contract_b_0: ChildContract,
        contract_b_1: ChildContract,
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractParams {
        contract_b_0_params: ChildContractParams,
        contract_b_1_params: ChildContractParams,
    }
    impl ComposableParameters for ParentContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            unimplemented!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractSummary;
    impl<'a> From<&'a ParentContract> for ParentContractSummary {
        fn from(value: &'a ParentContract) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a ParentContractSummary> for ChildContractSummary {
        fn from(value: &'a ParentContractSummary) -> Self {
            todo!()
        }
    }
    impl ComposableSummary<ChildContractSummary> for ParentContractSummary {
        fn merge(&mut self, _value: ChildContractSummary) {
            todo!()
        }
    }
    impl ComposableSummary<ParentContractSummary> for ParentContractSummary {
        fn merge(&mut self, _value: ParentContractSummary) {
            todo!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractDelta {
        contract_b_0: ChildContractDelta,
        contract_b_1: ChildContractDelta,
    }
    impl<'a> From<&'a ParentContract> for ChildContract {
        fn from(value: &'a ParentContract) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a ParentContractDelta> for ChildContractDelta {
        fn from(value: &'a ParentContractDelta) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a ParentContractParams> for ChildContractParams {
        fn from(value: &'a ParentContractParams) -> Self {
            todo!()
        }
    }

    #[contract(children(ChildContract, ChildContract))]
    // todo: this would be derived ideally
    impl ComposableContract for ParentContract {
        type Context = NoContext;
        type Parameters = ParentContractParams;
        type Delta = ParentContractDelta;
        type Summary = ParentContractSummary;

        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildContract as ComposableContract>::verify::<ChildContract, Self>(
                &self.contract_b_0,
                &<ChildContract as ComposableContract>::Parameters::from(parameters),
                self,
                related,
            )?;
            Ok(ValidateResult::Valid)
        }

        fn verify_delta<Child>(
            parameters: &Self::Parameters,
            delta: &Self::Delta,
        ) -> Result<bool, ContractError>
        where
            Child: ComposableContract,
        {
            <ChildContract as ComposableContract>::verify_delta::<ChildContract>(
                &parameters.into(),
                &delta.into(),
            )?;
            Ok(true)
        }

        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            update_data: &TypedUpdateData<Self>,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            {
                let sub_update: TypedUpdateData<ChildContract> =
                    TypedUpdateData::from_other(update_data);
                match freenet_stdlib::composers::ComposableContract::merge(
                    &mut self.contract_b_0,
                    &parameters.into(),
                    &sub_update,
                    related,
                ) {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => return MergeResult::RequestRelated(req),
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            {
                let sub_update: TypedUpdateData<ChildContract> =
                    TypedUpdateData::from_other(update_data);
                match freenet_stdlib::composers::ComposableContract::merge(
                    &mut self.contract_b_1,
                    &parameters.into(),
                    &sub_update,
                    related,
                ) {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => return MergeResult::RequestRelated(req),
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            MergeResult::Success
        }

        fn summarize<ParentSummary>(
            &self,
            parameters: &Self::Parameters,
            summary: &mut ParentSummary,
        ) -> Result<(), ContractError>
        where
            ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>,
        {
            self.contract_b_0
                .summarize(&parameters.into(), &mut ParentContractSummary)?;
            self.contract_b_1
                .summarize(&parameters.into(), &mut ParentContractSummary)?;
            Ok(())
        }

        fn delta(
            &self,
            parameters: &Self::Parameters,
            summary: &Self::Summary,
        ) -> Result<Self::Delta, ContractError> {
            let contract_b_0 = self
                .contract_b_0
                .delta(&parameters.into(), &summary.into())?;
            let contract_b_1 = self
                .contract_b_0
                .delta(&parameters.into(), &summary.into())?;
            Ok(ParentContractDelta {
                contract_b_0,
                contract_b_1,
            })
        }
    }

    impl<'x> From<&'x ParentContract> for children::PubKey {
        fn from(value: &'x ParentContract) -> Self {
            children::PubKey
        }
    }
}

mod children {
    use std::error::Error;

    use freenet_stdlib::{composers::*, prelude::*};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct ChildContract {}

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ChildContractParams;
    impl ComposableParameters for ChildContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            unimplemented!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ChildContractSummary;

    #[derive(Serialize, Deserialize)]
    pub struct ChildContractDelta;

    pub struct PubKey;

    impl From<ChildContractParams> for PubKey {
        fn from(value: ChildContractParams) -> Self {
            PubKey
        }
    }

    impl ComposableContract for ChildContract {
        type Context = PubKey;
        type Summary = ChildContractSummary;
        type Parameters = ChildContractParams;
        type Delta = ChildContractDelta;

        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(ValidateResult::Valid)
        }

        fn verify_delta<Child>(
            parameters: &Self::Parameters,
            delta: &Self::Delta,
        ) -> Result<bool, ContractError>
        where
            Child: ComposableContract,
        {
            let pub_key = PubKey::from(parameters.clone());
            Ok(true)
        }

        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            _delta: &TypedUpdateData<Self>,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            let contract_id = parameters.contract_id().unwrap();
            let Related::Found {
                state: mut contract_b,
                ..
            } = related.get::<ChildContract>(&contract_id)
            else {
                let mut req = RelatedContractsContainer::default();
                req.request::<ChildContract>(contract_id);
                return MergeResult::RequestRelated(req);
            };
            MergeResult::Success
        }

        fn delta(
            &self,
            _parameters: &Self::Parameters,
            _summary: &Self::Summary,
        ) -> Result<Self::Delta, ContractError> {
            todo!()
        }

        fn summarize<ParentSummary>(
            &self,
            _parameters: &Self::Parameters,
            summary: &mut ParentSummary,
        ) -> Result<(), ContractError>
        where
            ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>,
        {
            summary.merge(ChildContractSummary);
            todo!()
        }
    }
}
