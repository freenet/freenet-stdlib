use crate::contract_interface::{
    ContractError, ContractInstanceId, RelatedContracts, State, UpdateData, ValidateResult,
};

impl<'a> From<&'a State<'static>> for State<'static> {
    fn from(value: &'a State<'static>) -> Self {
        value.clone()
    }
}

pub trait ComposableParameters {
    fn contract_id(&self) -> Option<ContractInstanceId>;
}

pub trait ComposableContract: std::any::Any + Sized {
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
    ) -> Result<ValidateResult, ContractError>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        Self::Context: for<'x> From<&'x Ctx>;

    /// Corresponds to ContractInterface `validate_delta`
    fn verify_delta<Child>(
        parameters: &Self::Parameters,
        delta: &Self::Delta,
    ) -> Result<bool, ContractError>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>;

    /// Corresponds to ContractInterface `update_state`
    fn merge<Child>(
        &mut self,
        _parameters: &Self::Parameters,
        _delta: &TypedUpdateData<Self>,
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
    ) -> Result<Self::Summary, ContractError>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>;

    /// Corresponds to ContractInterface `delta`
    fn delta<Child>(
        &self,
        parameters: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, ContractError>
    where
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Child as ComposableContract>::Summary: for<'x> From<&'x Self::Summary>;
}

pub struct NoChild;

pub enum TypedUpdateData<T: ComposableContract> {
    RelatedState { state: T },
    RelatedDelta { delta: T::Delta },
    RelatedStateAndDelta { state: T, delta: T::Delta },
}

impl<T: ComposableContract> TypedUpdateData<T> {
    pub fn from_other<Parent>(_value: &TypedUpdateData<Parent>) -> Self
    where
        Parent: ComposableContract,
        <T as ComposableContract>::Delta: for<'x> From<&'x Parent::Delta>,
        T: for<'x> From<&'x Parent>,
    {
        todo!()
    }
}

impl<T: ComposableContract> From<(Option<T>, Option<T::Delta>)> for TypedUpdateData<T> {
    fn from((_state, _delta): (Option<T>, Option<T::Delta>)) -> Self {
        todo!()
    }
}

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
    ) -> Result<ValidateResult, ContractError>
    where
        Children: ComposableContract,
        <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        Self::Context: for<'x> From<&'x Ctx>,
    {
        Ok(ValidateResult::Valid)
    }

    fn verify_delta<Children>(
        _parameters: &Self::Parameters,
        _delta: &Self::Delta,
    ) -> Result<bool, ContractError>
    where
        Children: ComposableContract,
        <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
        <Children as ComposableContract>::Delta: for<'x> From<&'x Self::Delta>,
    {
        Ok(true)
    }

    fn merge<Child>(
        &mut self,
        _parameters: &Self::Parameters,
        _delta: &TypedUpdateData<Self>,
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
    ) -> Result<Self::Delta, ContractError>
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
    ) -> Result<Self::Summary, ContractError>
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

pub struct NoContext;

impl<'x, T> From<&'x T> for NoContext {
    fn from(_: &'x T) -> Self {
        NoContext
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
    Error(ContractError),
}

#[derive(Default)]
pub struct RelatedContractsContainer {}

impl From<RelatedContracts<'static>> for RelatedContractsContainer {
    fn from(_value: RelatedContracts<'static>) -> Self {
        todo!()
    }
}

impl From<RelatedContractsContainer> for Vec<crate::contract_interface::RelatedContract> {
    fn from(_value: RelatedContractsContainer) -> Self {
        todo!()
    }
}

impl From<Vec<UpdateData<'static>>> for RelatedContractsContainer {
    fn from(_value: Vec<UpdateData<'static>>) -> Self {
        todo!()
    }
}

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

pub mod from_bytes {
    use crate::{
        contract_interface::{
            serialization::{Encoder, SerializationAdapter},
            StateDelta, StateSummary, UpdateModification,
        },
        parameters::Parameters,
    };

    use super::*;

    pub fn inner_validate_state<T, Child, Ctx>(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as ComposableContract>::Parameters: Encoder,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        for<'x> <T as ComposableContract>::Context: From<&'x Ctx>,
        ContractError: From<<<T as SerializationAdapter>::Parameters as Encoder>::Error>,
        ContractError: From<<T as Encoder>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Context: for<'x> From<&'x T>,
        Ctx: for<'x> From<&'x T>,
    {
        let typed_params: <T as ComposableContract>::Parameters =
            <<T as SerializationAdapter>::Parameters as Encoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T = <T as Encoder>::deserialize(state.as_ref())?;
        let related_container = RelatedContractsContainer::from(related);
        let ctx = Ctx::from(&typed_state);
        match typed_state.verify::<Child, Ctx>(&typed_params, &ctx, &related_container)? {
            ValidateResult::Valid => {}
            ValidateResult::Invalid => return Ok(ValidateResult::Invalid),
            ValidateResult::RequestRelated(related) => {
                return Ok(ValidateResult::RequestRelated(related))
            }
        }
        Ok(ValidateResult::Valid)
    }

    pub fn inner_validate_delta<T, Child>(
        parameters: Parameters<'static>,
        delta: StateDelta<'static>,
    ) -> Result<bool, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Delta: Encoder,
        <T as ComposableContract>::Parameters: Encoder,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ComposableContract>::Delta>,
        ContractError: From<<<T as SerializationAdapter>::Parameters as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Delta as Encoder>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x <T as ComposableContract>::Delta>,
    {
        let typed_params =
            <<T as SerializationAdapter>::Parameters as Encoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_delta =
            <<T as SerializationAdapter>::Delta as Encoder>::deserialize(delta.as_ref())?.into();
        <T as ComposableContract>::verify_delta::<Child>(&typed_params, &typed_delta)
    }

    pub fn inner_update_state<T, Child>(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Delta: Encoder,
        <T as ComposableContract>::Parameters: Encoder,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ComposableContract>::Delta>,
        ContractError: From<<<T as SerializationAdapter>::Parameters as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Delta as Encoder>::Error>,
        ContractError: From<<T as Encoder>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x <T as ComposableContract>::Delta>,
    {
        let typed_params =
            <<T as SerializationAdapter>::Parameters as Encoder>::deserialize(parameters.as_ref())?
                .into();
        let mut typed_state: T = <T as Encoder>::deserialize(state.as_ref())?;
        let self_updates = UpdateData::get_self_states(&data);
        let related_container = RelatedContractsContainer::from(data);
        for (state, delta) in self_updates {
            let state = state
                .map(|s| <T as Encoder>::deserialize(s.as_ref()).map(Into::into))
                .transpose()?;
            let delta = delta
                .map(|d| {
                    <<T as SerializationAdapter>::Delta as Encoder>::deserialize(d.as_ref())
                        .map(Into::into)
                })
                .transpose()?;
            let typed_update = TypedUpdateData::from((state, delta));
            match typed_state.merge::<Child>(&typed_params, &typed_update, &related_container) {
                MergeResult::Success => {}
                MergeResult::RequestRelated(req) => {
                    return Ok(UpdateModification::requires(req.into()))
                }
                MergeResult::Error(err) => return Err(err),
            }
        }
        let encoded = typed_state.serialize()?;
        Ok(UpdateModification::valid(encoded.into()))
    }

    pub fn inner_summarize_state<T, Child>(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Summary: From<<T as ComposableContract>::Summary>,
        ContractError: From<<<T as SerializationAdapter>::Parameters as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Summary as Encoder>::Error>,
        ContractError: From<<T as Encoder>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
    {
        let typed_params =
            <<T as SerializationAdapter>::Parameters as Encoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T = <T as Encoder>::deserialize(state.as_ref())?;
        let summary = typed_state.summarize::<Child>(&typed_params)?;
        let encoded_summary = <T as SerializationAdapter>::Summary::from(summary).serialize()?;
        Ok(encoded_summary.into())
    }

    pub fn inner_state_delta<T, Child>(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Summary: Into<<T as ComposableContract>::Summary>,
        <T as SerializationAdapter>::Delta: From<<T as ComposableContract>::Delta>,
        ContractError: From<<T as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Parameters as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Summary as Encoder>::Error>,
        ContractError: From<<<T as SerializationAdapter>::Delta as Encoder>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Summary:
            for<'x> From<&'x <T as ComposableContract>::Summary>,
    {
        let typed_params =
            <<T as SerializationAdapter>::Parameters as Encoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T = <T as Encoder>::deserialize(state.as_ref())?;
        let typed_summary =
            <<T as SerializationAdapter>::Summary as Encoder>::deserialize(summary.as_ref())?
                .into();
        let delta = typed_state.delta::<Child>(&typed_params, &typed_summary)?;
        let encoded_delta = <T as SerializationAdapter>::Delta::from(delta).serialize()?;
        Ok(encoded_delta.into())
    }
}
