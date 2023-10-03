#![allow(dead_code, unused)]
use std::error::Error;

mod parent {
    use std::error::Error;

    use super::children::{self, *};
    use freenet_stdlib::{composers::*, prelude::*};

    pub struct ParentContract {
        contract_b_0: ChildrenContract,
        contract_b_1: ChildrenContract,
    }

    pub struct ParentContractParams {
        contract_b_0_params: ChildrenContractParams,
        contract_b_1_params: ChildrenContractParams,
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

    pub struct ParentContractSummary;
    impl<'a> From<&'a StateSummary<'_>> for ParentContractSummary {
        fn from(value: &'a StateSummary<'_>) -> Self {
            todo!()
        }
    }

    pub struct ParentContractDelta {
        contract_b_0: ChildrenContractDelta,
        contract_b_1: ChildrenContractDelta,
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

    impl<'a> From<&'a ParentContract> for ChildrenContract {
        fn from(value: &'a ParentContract) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a ParentContractDelta> for ChildrenContractDelta {
        fn from(value: &'a ParentContractDelta) -> Self {
            todo!()
        }
    }
    impl<'a> From<&'a ParentContractParams> for ChildrenContractParams {
        fn from(value: &'a ParentContractParams) -> Self {
            todo!()
        }
    }

    // todo: this would be derived ideally
    impl ComposableContract for ParentContract {
        type Context = Self;
        type Parameters = ParentContractParams;
        type Delta = ParentContractDelta;
        type Summary = ParentContractSummary;

        fn verify<Children, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn std::error::Error>>
        where
            Children: ComposableContract,
            <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildrenContract as ComposableContract>::verify::<NoChild, Self>(
                &self.contract_b_0,
                &parameters.into(),
                self,
                related,
            )?;
            Ok(())
        }

        fn verify_delta<Children, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), Box<dyn Error>>
        where
            Children: ComposableContract,
            <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Children as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildrenContract as ComposableContract>::verify_delta::<NoChild, Self>(
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

    pub struct ChildrenContract {}

    pub struct ChildrenContractParams;
    impl ComposableParameters for ChildrenContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            unimplemented!()
        }
    }

    pub struct ChildrenContractSummary;

    pub struct ChildrenContractDelta;

    pub struct PubKey;

    impl ComposableContract for ChildrenContract {
        type Context = PubKey;
        type Summary = ChildrenContractSummary;
        type Parameters = ChildrenContractParams;
        type Delta = ChildrenContractDelta;

        fn verify<Children, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn std::error::Error>>
        where
            Children: ComposableContract,
            <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(())
        }

        fn verify_delta<Children, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), Box<dyn Error>>
        where
            Children: ComposableContract,
            <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            <Children as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
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
            } = related.get::<ChildrenContract>(&contract_id)
            else {
                let mut req = RelatedContractsContainer::default();
                req.request::<ChildrenContract>(contract_id);
                return MergeResult::RequestRelated(req);
            };
            MergeResult::Success
        }
    }
}

fn main() {}
