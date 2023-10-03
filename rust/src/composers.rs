use std::error::Error;

use crate::contract_interface::{ContractInstanceId, State};

impl<'a> From<&'a State<'static>> for State<'static> {
    fn from(value: &'a State<'static>) -> Self {
        value.clone()
    }
}

pub trait ComposableParameters {
    fn contract_id(&self) -> Option<ContractInstanceId>;
}

pub trait ComposableContract: std::any::Any {
    type Context;
    type Parameters: ComposableParameters;
    type Delta;
    type Summary;

    /// Corresponds to ContractInterface `validate_state`
    fn verify<Child, Ctx>(
        &self,
        parameters: &Self::Parameters,
        context: &Ctx,
        related: &RelatedContractsContainer,
    ) -> Result<(), Box<dyn Error>>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        Self::Context: for<'x> From<&'x Ctx>;

    /// Corresponds to ContractInterface `validate_delta`
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
        Self::Context: for<'x> From<&'x Ctx>;

    /// Corresponds to ContractInterface `update_state`
    fn merge<Child>(
        &mut self,
        _parameters: &Self::Parameters,
        _delta: &Self::Delta,
        _related: &RelatedContractsContainer,
    ) -> MergeResult
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>;

    /// Corresponds to ContractInterface `summarize`
    fn summarize<Child>(
        &self,
        parameters: &Self::Parameters,
    ) -> Result<Self::Summary, Box<dyn Error>>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>;

    /// Corresponds to ContractInterface `delta`
    fn delta<Child>(
        &self,
        parameters: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, Box<dyn Error>>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Child as ComposableContract>::Summary: for<'x> From<&'x Self::Summary>;
}

pub struct NoChild;

impl ComposableContract for NoChild {
    type Parameters = NoChild;
    type Context = NoChild;
    type Delta = NoChild;
    type Summary = NoChild;

    fn verify<Children, Ctx>(
        &self,
        _parameters: &Self::Parameters,
        _context: &Ctx,
        _related: &RelatedContractsContainer,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        Children: ComposableContract,
        <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        Self::Context: for<'x> From<&'x Ctx>,
    {
        Ok(())
    }

    fn verify_delta<Children, Ctx>(
        &self,
        _parameters: &Self::Parameters,
        _context: &Ctx,
        _delta: &Self::Delta,
    ) -> Result<(), Box<dyn Error>>
    where
        Children: ComposableContract,
        <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Children as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
        Self::Context: for<'x> From<&'x Ctx>,
    {
        Ok(())
    }

    fn merge<Child>(
        &mut self,
        _parameters: &Self::Parameters,
        _delta: &Self::Delta,
        _related: &RelatedContractsContainer,
    ) -> MergeResult
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
    {
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
        Ok(NoChild)
    }

    fn summarize<Child>(
        &self,
        _parameters: &Self::Parameters,
    ) -> Result<Self::Summary, Box<dyn Error>>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
    {
        Ok(NoChild)
    }
}

impl ComposableParameters for NoChild {
    fn contract_id(&self) -> Option<ContractInstanceId> {
        None
    }
}

impl<'x, T> From<&'x T> for NoChild {
    fn from(_: &'x T) -> Self {
        NoChild
    }
}

pub enum Related<C: ComposableContract> {
    /// The state was previously requested and found
    Found { state: C },
    /// The state was previously requested but not found
    NotFound,
    /// The state was previously requested but request is still in flight
    RequestPending,
    /// The state was not previously requested, this enum can be included
    /// in the MergeResult return value which will request it
    NotRequested,
}

pub enum MergeResult {
    Success,
    RequestRelated(RelatedContractsContainer),
    Error(Box<dyn Error>),
}

#[derive(Default)]
pub struct RelatedContractsContainer {}

impl RelatedContractsContainer {
    pub fn get<C: ComposableContract>(&self, _id: &ContractInstanceId) -> Related<C> {
        todo!()
    }

    pub fn request<C: ComposableContract>(&mut self, _request: ContractInstanceId) {
        todo!()
    }

    pub fn merge(&mut self, _other: Self) {
        todo!()
    }
}
