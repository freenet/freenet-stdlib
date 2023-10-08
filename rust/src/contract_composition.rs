use crate::{
    contract_interface::{
        ContractError, ContractInstanceId, RelatedContracts, State, UpdateData, ValidateResult,
    },
    typed_contract::{MergeResult, RelatedContractsContainer},
};

impl<'a> From<&'a State<'static>> for State<'static> {
    fn from(value: &'a State<'static>) -> Self {
        value.clone()
    }
}

pub trait ParametersComponent {
    fn contract_id(&self) -> Option<ContractInstanceId>;
}

pub trait SummaryComponent<ChildSummary> {
    fn merge(&mut self, _child_summary: ChildSummary);
}

pub trait ContractComponent: std::any::Any + Sized {
    type Context;
    type Parameters: ParametersComponent;
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
        Child: ContractComponent,
        Self::Context: for<'x> From<&'x Ctx>;

    /// Corresponds to ContractInterface `validate_delta`
    fn verify_delta<Child>(
        parameters: &Self::Parameters,
        delta: &Self::Delta,
    ) -> Result<bool, ContractError>
    where
        Child: ContractComponent;

    /// Corresponds to ContractInterface `update_state`
    fn merge(
        &mut self,
        parameters: &Self::Parameters,
        update: &TypedUpdateData<Self>,
        related: &RelatedContractsContainer,
    ) -> MergeResult;

    /// Corresponds to ContractInterface `summarize`
    fn summarize<ParentSummary>(
        &self,
        parameters: &Self::Parameters,
        summary: &mut ParentSummary,
    ) -> Result<(), ContractError>
    where
        ParentSummary: SummaryComponent<<Self as ContractComponent>::Summary>;

    /// Corresponds to ContractInterface `delta`
    fn delta(
        &self,
        parameters: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, ContractError>;
}

pub enum TypedUpdateData<T: ContractComponent> {
    RelatedState { state: T },
    RelatedDelta { delta: T::Delta },
    RelatedStateAndDelta { state: T, delta: T::Delta },
}

impl<T: ContractComponent> TypedUpdateData<T> {
    pub fn from_other<Parent>(_value: &TypedUpdateData<Parent>) -> Self
    where
        Parent: ContractComponent,
        <T as ContractComponent>::Delta: for<'x> From<&'x Parent::Delta>,
        T: for<'x> From<&'x Parent>,
    {
        todo!()
    }
}

impl<T: ContractComponent> From<(Option<T>, Option<T::Delta>)> for TypedUpdateData<T> {
    fn from((_state, _delta): (Option<T>, Option<T::Delta>)) -> Self {
        todo!()
    }
}

#[allow(unused)]
impl<T: ContractComponent> ContractComponent for Vec<T> {
    type Context = T::Context;
    type Parameters = T::Parameters;
    type Delta = T::Delta;
    type Summary = T::Summary;

    fn verify<Child, Ctx>(
        &self,
        parameters: &Self::Parameters,
        context: &Ctx,
        related: &RelatedContractsContainer,
    ) -> Result<ValidateResult, ContractError>
    where
        Child: ContractComponent,
        Self::Context: for<'x> From<&'x Ctx>,
    {
        todo!()
    }

    fn verify_delta<Child>(
        parameters: &Self::Parameters,
        delta: &Self::Delta,
    ) -> Result<bool, ContractError>
    where
        Child: ContractComponent,
    {
        todo!()
    }

    fn merge(
        &mut self,
        parameters: &Self::Parameters,
        update: &TypedUpdateData<Self>,
        related: &RelatedContractsContainer,
    ) -> MergeResult {
        todo!()
    }

    fn summarize<ParentSummary>(
        &self,
        parameters: &Self::Parameters,
        summary: &mut ParentSummary,
    ) -> Result<(), ContractError>
    where
        ParentSummary: SummaryComponent<<Self as ContractComponent>::Summary>,
    {
        todo!()
    }

    fn delta(
        &self,
        parameters: &Self::Parameters,
        summary: &Self::Summary,
    ) -> Result<Self::Delta, ContractError> {
        todo!()
    }
}

pub struct NoContext;

impl<'x, T> From<&'x T> for NoContext {
    fn from(_: &'x T) -> Self {
        NoContext
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

    pub fn inner_validate_state<T, Child, Ctx>(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>
    where
        T: ContractComponent + SerializationAdapter + DeserializeOwned,
        <T as SerializationAdapter>::Parameters: Into<<T as ContractComponent>::Parameters>,
        for<'x> <T as ContractComponent>::Context: From<&'x Ctx>,
        ContractError: From<
            <<T as SerializationAdapter>::ParametersEncoder as Encoder<
                <T as SerializationAdapter>::Parameters,
            >>::Error,
        >,
        ContractError: From<<<T as SerializationAdapter>::SelfEncoder as Encoder<T>>::Error>,
        Child: ContractComponent,
        <Child as ContractComponent>::Parameters:
            for<'x> From<&'x <T as ContractComponent>::Parameters>,
        <Child as ContractComponent>::Context: for<'x> From<&'x T>,
        Ctx: for<'x> From<&'x T>,
    {
        let typed_params: <T as ContractComponent>::Parameters =
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
        T: ContractComponent + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ContractComponent>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ContractComponent>::Delta>,
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
        Child: ContractComponent,
        <Child as ContractComponent>::Parameters:
            for<'x> From<&'x <T as ContractComponent>::Parameters>,
        <Child as ContractComponent>::Delta: for<'x> From<&'x <T as ContractComponent>::Delta>,
    {
        let typed_params =
            <<T as SerializationAdapter>::ParametersEncoder>::deserialize(parameters.as_ref())?
                .into();
        let typed_delta =
            <<T as SerializationAdapter>::DeltaEncoder>::deserialize(delta.as_ref())?.into();
        <T as ContractComponent>::verify_delta::<Child>(&typed_params, &typed_delta)
    }

    pub fn inner_update_state<T, Child>(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError>
    where
        T: ContractComponent + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ContractComponent>::Parameters>,
        <T as SerializationAdapter>::Delta: Into<<T as ContractComponent>::Delta>,
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
        Child: ContractComponent,
        <Child as ContractComponent>::Parameters:
            for<'x> From<&'x <T as ContractComponent>::Parameters>,
        <Child as ContractComponent>::Delta: for<'x> From<&'x <T as ContractComponent>::Delta>,
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
    ) -> Result<<T as ContractComponent>::Summary, ContractError>
    where
        T: ContractComponent + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ContractComponent>::Parameters>,
        <T as ContractComponent>::Summary:
            for<'x> From<&'x T> + SummaryComponent<<T as ContractComponent>::Summary>,
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
        let mut summary = <<T as ContractComponent>::Summary>::from(&typed_state);
        typed_state.summarize(&typed_params, &mut summary)?;
        Ok(summary)
    }

    pub fn inner_state_delta<T>(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<<T as ContractComponent>::Delta, ContractError>
    where
        T: ContractComponent + SerializationAdapter,
        <T as SerializationAdapter>::Parameters: Into<<T as ContractComponent>::Parameters>,
        <T as SerializationAdapter>::Summary: Into<<T as ContractComponent>::Summary>,
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
