#![allow(dead_code, unused)]
use std::error::Error;

mod parent {
    use std::error::Error;

    use super::children::{self, *};
    use freenet_stdlib::{composers::*, prelude::*};

    pub struct ParentContract {
        contract_b_0: ChildContract,
        contract_b_1: ChildContract,
    }

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

    pub struct ParentContractSummary {
        child_0_summary: ChildContractSummary,
        child_1_summary: ChildContractSummary,
    }
    impl<'a> From<&'a StateSummary<'_>> for ParentContractSummary {
        fn from(value: &'a StateSummary<'_>) -> Self {
            todo!()
        }
    }

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
        type Context = Self;
        type Parameters = ParentContractParams;
        type Delta = ParentContractDelta;
        type Summary = ParentContractSummary;

        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn std::error::Error>>
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
            Ok(())
        }

        fn verify_delta<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), Box<dyn Error>>
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

    pub struct ChildContract {}

    pub struct ChildContractParams;
    impl ComposableParameters for ChildContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            unimplemented!()
        }
    }

    pub struct ChildContractSummary;

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
        ) -> Result<(), Box<dyn std::error::Error>>
        where
            Child: ComposableContract,
            <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(())
        }

        fn verify_delta<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            delta: &Self::Delta,
        ) -> Result<(), Box<dyn Error>>
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
