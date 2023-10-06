mod parent {
    //! This would be the end crate.

    // children would be like a different dependency crate which implements composable types
    use super::children::{self, *};

    use freenet_stdlib::{composers::*, prelude::*};
    use serde::{Deserialize, Serialize};

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
        fn from(_: &'a ParentContract) -> Self {
            unimplemented!()
        }
    }
    impl<'a> From<&'a ParentContractSummary> for ChildContractSummary {
        fn from(_: &'a ParentContractSummary) -> Self {
            unimplemented!()
        }
    }
    impl ComposableSummary<ChildContractSummary> for ParentContractSummary {
        fn merge(&mut self, _: ChildContractSummary) {
            unimplemented!()
        }
    }
    impl ComposableSummary<ParentContractSummary> for ParentContractSummary {
        fn merge(&mut self, _: ParentContractSummary) {
            unimplemented!()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ParentContractDelta {
        contract_b_0: ChildContractDelta,
        contract_b_1: ChildContractDelta,
    }
    impl<'a> From<&'a ParentContract> for ChildContract {
        fn from(_: &'a ParentContract) -> Self {
            unimplemented!()
        }
    }
    impl<'a> From<&'a ParentContractDelta> for ChildContractDelta {
        fn from(_: &'a ParentContractDelta) -> Self {
            unimplemented!()
        }
    }
    impl<'a> From<&'a ParentContractParams> for ChildContractParams {
        fn from(_: &'a ParentContractParams) -> Self {
            unimplemented!()
        }
    }

    #[contract(children(ChildContract, ChildContract), encoder = BincodeEncoder)]
    // todo: this impl block would be derived ideally, we can have a derive macro
    // in the struct where the associated types need to be specified
    impl ComposableContract for ParentContract {
        type Context = NoContext;
        type Parameters = ParentContractParams;
        type Delta = ParentContractDelta;
        type Summary = ParentContractSummary;

        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            _ctx: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildContract as ComposableContract>::verify::<ChildContract, _>(
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
            // todo: probably need ParentSummary to impl From<&Self>?
            let mut this_summary = ParentContractSummary;
            self.contract_b_0
                .summarize(&parameters.into(), &mut this_summary)?;
            self.contract_b_1
                .summarize(&parameters.into(), &mut this_summary)?;
            summary.merge(this_summary);
            Ok(())
        }

        fn delta(
            &self,
            parameters: &Self::Parameters,
            summary: &Self::Summary,
        ) -> Result<Self::Delta, ContractError> {
            // todo: this impl may be probematic to derive, specially getting the return type
            // maybe requires adding an other transformation bound
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
        fn from(_: &'x ParentContract) -> Self {
            children::PubKey
        }
    }
}

mod children {
    //! This would be a depebdency crate.

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
        fn from(_: ChildContractParams) -> Self {
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
            _parameters: &Self::Parameters,
            context: &Ctx,
            _related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let _pub_key = PubKey::from(context);
            // assert something in self/context is signed with pub key
            Ok(ValidateResult::Valid)
        }

        fn verify_delta<Child>(
            parameters: &Self::Parameters,
            _delta: &Self::Delta,
        ) -> Result<bool, ContractError>
        where
            Child: ComposableContract,
        {
            let _pub_key = PubKey::from(parameters.clone());
            // assert something in Self::Delta is signed with pub key
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
                state: _other_contract,
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
            Ok(ChildContractDelta)
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
            Ok(())
        }
    }
}
