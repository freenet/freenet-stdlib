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
            type State = ParentContract;
            type StateDelta = ParentContractDelta;
            type StateSummary = ParentContractSummary;
        }

        // todo: have an other macro that can be snapped on top of `impl SerializationAdapter`
        // that generated all of below, it cna take a parameter (e.g. BincodeEncoder) to specify
        // the encoder to use
        impl BincodeEncoder for ParentContract {}
        impl BincodeEncoder for ParentContractParams {}
        impl BincodeEncoder for ParentContractDelta {}
        impl BincodeEncoder for ParentContractSummary {}

        #[no_mangle]
        pub extern "C" fn validate_state(parameters: i64, state: i64, related: i64) -> i64 {
            ::freenet_stdlib::memory::wasm_interface::inner_validate_state::<ParentContract>(
                parameters, state, related,
            )
        }

        impl ContractInterface for ParentContract {
            fn validate_state(
                parameters: freenet_stdlib::prelude::Parameters<'static>,
                state: freenet_stdlib::prelude::State<'static>,
                related: freenet_stdlib::prelude::RelatedContracts<'static>,
            ) -> Result<
                freenet_stdlib::prelude::ValidateResult,
                freenet_stdlib::prelude::ContractError,
            > {
                let typed_params =
                    <<Self as SerializationAdapter>::Parameters as Encoder>::deserialize(
                        parameters.as_ref(),
                    )?;
                let typed_state = <<Self as SerializationAdapter>::State as Encoder>::deserialize(
                    state.as_ref(),
                )?;
                let related_container = RelatedContractsContainer::from(related);
                match typed_state.verify::<NoChild, NoContext>(
                    &typed_params,
                    &NoContext,
                    &related_container,
                )? {
                    ValidateResult::Valid => {}
                    ValidateResult::Invalid => return Ok(ValidateResult::Invalid),
                    ValidateResult::RequestRelated(related) => {
                        return Ok(ValidateResult::RequestRelated(related))
                    }
                }
                Ok(ValidateResult::Valid)
            }

            fn validate_delta(
                parameters: freenet_stdlib::prelude::Parameters<'static>,
                delta: freenet_stdlib::prelude::StateDelta<'static>,
            ) -> Result<bool, freenet_stdlib::prelude::ContractError> {
                todo!()
            }

            fn update_state(
                parameters: freenet_stdlib::prelude::Parameters<'static>,
                state: freenet_stdlib::prelude::State<'static>,
                data: Vec<freenet_stdlib::prelude::UpdateData<'static>>,
            ) -> Result<
                freenet_stdlib::prelude::UpdateModification<'static>,
                freenet_stdlib::prelude::ContractError,
            > {
                todo!()
            }

            fn summarize_state(
                parameters: freenet_stdlib::prelude::Parameters<'static>,
                state: freenet_stdlib::prelude::State<'static>,
            ) -> Result<
                freenet_stdlib::prelude::StateSummary<'static>,
                freenet_stdlib::prelude::ContractError,
            > {
                todo!()
            }

            fn get_state_delta(
                parameters: freenet_stdlib::prelude::Parameters<'static>,
                state: freenet_stdlib::prelude::State<'static>,
                summary: freenet_stdlib::prelude::StateSummary<'static>,
            ) -> Result<
                freenet_stdlib::prelude::StateDelta<'static>,
                freenet_stdlib::prelude::ContractError,
            > {
                todo!()
            }
        }
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
    impl<'a> From<&'a Parameters<'static>> for ParentContractParams {
        fn from(value: &'a Parameters<'static>) -> Self {
            todo!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractSummary {
        child_0_summary: ChildContractSummary,
        child_1_summary: ChildContractSummary,
    }
    impl<'a> From<&'a StateSummary<'_>> for ParentContractSummary {
        fn from(value: &'a StateSummary<'_>) -> Self {
            todo!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractDelta {
        contract_b_0: ChildContractDelta,
        contract_b_1: ChildContractDelta,
    }
    impl<'a> From<&'a StateDelta<'static>> for ParentContractDelta {
        fn from(_value: &'a StateDelta<'static>) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a State<'static>> for ParentContract {
        fn from(value: &'a State<'static>) -> Self {
            todo!()
        }
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
    impl<'a> From<&'a ParentContractSummary> for ChildContractSummary {
        fn from(value: &'a ParentContractSummary) -> Self {
            todo!()
        }
    }

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
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildContract as ComposableContract>::verify::<NoChild, Self>(
                &self.contract_b_0,
                &parameters.into(),
                self,
                related,
            )?;
            Ok(ValidateResult::Valid)
        }

        fn verify_delta<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), ContractError>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildContract as ComposableContract>::verify_delta::<NoChild, Self>(
                &self.contract_b_0,
                &parameters.into(),
                self,
                &delta.into(),
            )?;
            Ok(())
        }

        fn merge<Child>(
            &mut self,
            parameters: &Self::Parameters,
            delta: &Self::Delta,
            related: &RelatedContractsContainer,
        ) -> MergeResult
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
        {
            {
                match self
                    .contract_b_0
                    .merge::<NoChild>(&parameters.into(), &delta.into(), related)
                {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => return MergeResult::RequestRelated(req),
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            {
                match self
                    .contract_b_1
                    .merge::<NoChild>(&parameters.into(), &delta.into(), related)
                {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => return MergeResult::RequestRelated(req),
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            MergeResult::Success
        }

        fn delta<Child>(
            &self,
            parameters: &Self::Parameters,
            summary: &Self::Summary,
        ) -> Result<Self::Delta, Box<dyn Error>>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Summary: for<'x> From<&'x Self::Summary>,
        {
            let contract_b_0 = self
                .contract_b_0
                .delta::<NoChild>(&parameters.into(), &summary.into())?;
            let contract_b_1 = self
                .contract_b_0
                .delta::<NoChild>(&parameters.into(), &summary.into())?;
            Ok(ParentContractDelta {
                contract_b_0,
                contract_b_1,
            })
        }

        fn summarize<Child>(
            &self,
            parameters: &Self::Parameters,
        ) -> Result<Self::Summary, Box<dyn Error>>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        {
            let child_0_summary = self.contract_b_0.summarize::<NoChild>(&parameters.into())?;
            let child_1_summary = self.contract_b_1.summarize::<NoChild>(&parameters.into())?;
            Ok(ParentContractSummary {
                child_0_summary,
                child_1_summary,
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

    #[derive(Serialize, Deserialize)]
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
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(ValidateResult::Valid)
        }

        fn verify_delta<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), ContractError>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(())
        }

        fn merge<Child>(
            &mut self,
            parameters: &Self::Parameters,
            _delta: &Self::Delta,
            related: &RelatedContractsContainer,
        ) -> MergeResult
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
        {
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

        fn delta<Child>(
            &self,
            _parameters: &Self::Parameters,
            _summary: &Self::Summary,
        ) -> Result<Self::Delta, Box<dyn Error>>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Child as ComposableContract>::Summary: for<'x> From<&'x Self::Summary>,
        {
            todo!()
        }

        fn summarize<Child>(
            &self,
            _parameters: &Self::Parameters,
        ) -> Result<Self::Summary, Box<dyn Error>>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        {
            todo!()
        }
    }
}

fn main() {}
