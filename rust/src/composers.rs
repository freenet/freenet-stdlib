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

pub trait ComposableSummary<ChildSummary> {
    fn merge(&mut self, _child_summary: ChildSummary);
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
        Self::Context: for<'x> From<&'x Ctx>;

    /// Corresponds to ContractInterface `validate_delta`
    fn verify_delta<Child>(
        parameters: &Self::Parameters,
        delta: &Self::Delta,
    ) -> Result<bool, ContractError>
    where
        Child: ComposableContract;

    /// Corresponds to ContractInterface `update_state`
    fn merge(
        &mut self,
        _parameters: &Self::Parameters,
        _delta: &TypedUpdateData<Self>,
        _related: &RelatedContractsContainer,
    ) -> MergeResult;

    /// Corresponds to ContractInterface `summarize`
    fn summarize<ParentSummary>(
        &self,
        parameters: &Self::Parameters,
        summary: &mut ParentSummary,
    ) -> Result<(), ContractError>
    where
        ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>;

    /// Corresponds to ContractInterface `delta`
    fn delta(
        &self,
        parameters: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, ContractError>;
}

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

// pub struct NoChild;

// impl ComposableContract for NoChild {
//     type Parameters = NoChild;
//     type Context = NoChild;
//     type Delta = NoChild;
//     type Summary = NoChild;

//     fn verify<Children, Ctx>(
//         &self,
//         _parameters: &Self::Parameters,
//         _context: &Ctx,
//         _related: &RelatedContractsContainer,
//     ) -> Result<ValidateResult, ContractError>
//     where
//         Children: ComposableContract,
//         // <Children as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
//         Self::Context: for<'x> From<&'x Ctx>,
//     {
//         Ok(ValidateResult::Valid)
//     }

//     fn verify_delta<Children>(
//         _parameters: &Self::Parameters,
//         _delta: &Self::Delta,
//     ) -> Result<bool, ContractError>
//     where
//         Children: ComposableContract,
//     {
//         Ok(true)
//     }

//     fn merge(
//         &mut self,
//         _parameters: &Self::Parameters,
//         _delta: &TypedUpdateData<Self>,
//         _related: &RelatedContractsContainer,
//     ) -> MergeResult {
//         MergeResult::Success
//     }

//     fn delta(
//         &self,
//         _parameters: &Self::Parameters,
//         _summary: &Self::Summary,
//     ) -> Result<Self::Delta, ContractError> {
//         Ok(NoChild)
//     }

//     fn summarize<ParentSummary>(
//         &self,
//         _parameters: &Self::Parameters,
//         _summary: &mut ParentSummary,
//     ) -> Result<(), ContractError>
//     where
//         // <Child as ComposableContract>::Parameters: for<'x> From<&'x Self::Parameters>,
//         ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>,
//     {
//         Ok(())
//     }
// }

// impl ComposableParameters for NoChild {
//     fn contract_id(&self) -> Option<ContractInstanceId> {
//         None
//     }
// }

// impl<'x, T> From<&'x T> for NoChild {
//     fn from(_: &'x T) -> Self {
//         NoChild
//     }
// }

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
    use serde::de::DeserializeOwned;

    use crate::{
        contract_interface::{
            serialization::{Encoder, SerializationAdapter},
            StateDelta, StateSummary, UpdateModification,
        },
        parameters::Parameters,
    };

    use super::*;

    // <<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error
    pub fn inner_validate_state<T, Child, Ctx>(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>
    where
        T: ComposableContract + SerializationAdapter + DeserializeOwned,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        for<'x> <T as ComposableContract>::Context: From<&'x Ctx>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<<<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Context: for<'x> From<&'x T>,
        Ctx: for<'x> From<&'x T>,
    {
        let typed_params: <T as ComposableContract>::Parameters =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T =
            <<T as SerializationAdapter>::SelfEncoder>::deserialize(state.as_ref())?;
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
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ComposableContract>::Delta>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<
            <<T as SerializationAdapter>::DeltaEncoder as Encoder<
                <T as SerializationAdapter>::Delta,
            >>::Error,
        >,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x <T as ComposableContract>::Delta>,
    {
        let typed_params =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_delta =
            <<T as SerializationAdapter>::DeltaEncoder>::deserialize(delta.as_ref())?.into();
        <T as ComposableContract>::verify_delta::<Child>(&typed_params, &typed_delta)
    }

    pub fn inner_update_state<T, Child>(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ComposableContract>::Delta>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<
            <<T as SerializationAdapter>::DeltaEncoder as Encoder<
                <T as SerializationAdapter>::Delta,
            >>::Error,
        >,
        ContractError: From<<<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error>,
        Child: ComposableContract,
        <Child as ComposableContract>::Parameters:
            for<'x> From<&'x <T as ComposableContract>::Parameters>,
        <Child as ComposableContract>::Delta: for<'x> From<&'x <T as ComposableContract>::Delta>,
    {
        let typed_params =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let mut typed_state: T =
            <<T as SerializationAdapter>::SelfEncoder>::deserialize(state.as_ref())?;
        let self_updates = UpdateData::get_self_states(&data);
        let related_container = RelatedContractsContainer::from(data);
        for (state, delta) in self_updates {
            let state = state
                .map(|s| <<T as SerializationAdapter>::SelfEncoder>::deserialize(s.as_ref()))
                .transpose()?;
            let delta = delta
                .map(|d| {
                    <<T as SerializationAdapter>::DeltaEncoder>::deserialize(d.as_ref())
                        .map(Into::into)
                })
                .transpose()?;
            let typed_update = TypedUpdateData::from((state, delta));
            match typed_state.merge(&typed_params, &typed_update, &related_container) {
                MergeResult::Success => {}
                MergeResult::RequestRelated(req) => {
                    return UpdateModification::requires(req.into());
                }
                MergeResult::Error(err) => return Err(err),
            }
        }
        let encoded = <<T as SerializationAdapter>::SelfEncoder>::serialize(&typed_state)?;
        Ok(UpdateModification::valid(encoded.into()))
    }

    pub fn inner_summarize_state<T>(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<<T as ComposableContract>::Summary, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as ComposableContract>::Summary:
            for<'x> From<&'x T> + ComposableSummary<<T as ComposableContract>::Summary>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<<<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error>,
    {
        let typed_params =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T =
            <<T as SerializationAdapter>::SelfEncoder>::deserialize(state.as_ref())?;
        let mut summary = <<T as ComposableContract>::Summary>::from(&typed_state);
        typed_state.summarize(&typed_params, &mut summary)?;
        Ok(summary)
    }

    pub fn inner_state_delta<T>(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<<T as ComposableContract>::Delta, ContractError>
    where
        T: ComposableContract + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ComposableContract>::Parameters>,
        <T as SerializationAdapter>::Summary: Into<<T as ComposableContract>::Summary>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<
            <<T as SerializationAdapter>::SummaryEncoder as Encoder<
                <T as SerializationAdapter>::Summary,
            >>::Error,
        >,
        ContractError: From<<<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error>,
    {
        let typed_params =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_state: T =
            <<T as SerializationAdapter>::SelfEncoder>::deserialize(state.as_ref())?;
        let typed_summary =
            <<T as SerializationAdapter>::SummaryEncoder>::deserialize(summary.as_ref())?.into();
        typed_state.delta(&typed_params, &typed_summary)
    }
}
