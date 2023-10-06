use freenet_stdlib::composers::{ComposableContract, ComposableParameters};

pub struct ChatRoom {
    pub members: Vec<dependency_2::ChatRoomMember>,
    pub messages: Vec<dependency_1::SignedComposable<dependency_2::ChatRoomMessage>>,
}

pub struct ChatRoomParameters {
    pub owner_public_key: dependency_2::PublicKey,
}

impl ComposableParameters for ChatRoomParameters {
    fn contract_id(&self) -> Option<freenet_stdlib::prelude::ContractInstanceId> {
        unimplemented!()
    }
}

impl<'x> From<&'x ChatRoomParameters> for dependency_2::ChatRoomMsgParameters {
    fn from(_: &'x ChatRoomParameters) -> Self {
        unimplemented!()
    }
}

impl<'x> From<&'x ChatRoom> for dependency_2::PublicKey {
    fn from(_: &'x ChatRoom) -> Self {
        unimplemented!()
    }
}

impl ComposableContract for ChatRoom {
    type Context = ();
    type Parameters = ChatRoomParameters;
    type Delta = ();
    type Summary = ();

    fn verify<Child, Ctx>(
        &self,
        parameters: &Self::Parameters,
        _: &Ctx,
        related: &freenet_stdlib::prelude::RelatedContractsContainer,
    ) -> Result<freenet_stdlib::prelude::ValidateResult, freenet_stdlib::prelude::ContractError>
    where
        Child: ComposableContract,
        Self::Context: for<'x> From<&'x Ctx>,
    {
        self.messages
            .verify::<Vec<dependency_1::SignedComposable<dependency_2::ChatRoomMessage>>, _>(
                &parameters.into(),
                self,
                related,
            )?;
        unimplemented!()
    }

    fn verify_delta<Child>(
        _: &Self::Parameters,
        _: &Self::Delta,
    ) -> Result<bool, freenet_stdlib::prelude::ContractError>
    where
        Child: ComposableContract,
    {
        unimplemented!()
    }

    fn merge(
        &mut self,
        _: &Self::Parameters,
        _: &freenet_stdlib::composers::TypedUpdateData<Self>,
        _: &freenet_stdlib::prelude::RelatedContractsContainer,
    ) -> freenet_stdlib::composers::MergeResult {
        unimplemented!()
    }

    fn summarize<ParentSummary>(
        &self,
        _: &Self::Parameters,
        _: &mut ParentSummary,
    ) -> Result<(), freenet_stdlib::prelude::ContractError>
    where
        ParentSummary:
            freenet_stdlib::composers::ComposableSummary<<Self as ComposableContract>::Summary>,
    {
        unimplemented!()
    }

    fn delta(
        &self,
        _: &Self::Parameters,
        _: &Self::Summary,
    ) -> Result<Self::Delta, freenet_stdlib::prelude::ContractError> {
        unimplemented!()
    }
}

pub mod dependency_1 {
    pub struct Signature {}
    pub struct SignedComposable<S> {
        pub value: S,
        pub signature: Signature,
    }
}

pub mod dependency_2 {
    use freenet_stdlib::composers::{ComposableContract, ComposableParameters};

    #[derive(Clone, Copy)]
    pub struct PublicKey {}

    pub struct ChatRoomMember {
        pub name: String,
        pub public_key: PublicKey,
    }

    pub struct ChatRoomMessage {
        pub message: String,
        pub author: String,
    }

    pub struct ChatRoomMsgParameters {
        pub owner_public_key: PublicKey,
    }

    impl ComposableParameters for ChatRoomMsgParameters {
        fn contract_id(&self) -> Option<freenet_stdlib::prelude::ContractInstanceId> {
            unimplemented!()
        }
    }

    impl ComposableContract for super::dependency_1::SignedComposable<ChatRoomMessage> {
        type Context = PublicKey;
        type Parameters = ChatRoomMsgParameters;
        type Delta = ();
        type Summary = ();

        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            _: &freenet_stdlib::prelude::RelatedContractsContainer,
        ) -> Result<freenet_stdlib::prelude::ValidateResult, freenet_stdlib::prelude::ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let _public_key = parameters.owner_public_key;
            let _public_key = PublicKey::from(context);
            // do stuff with pub key
            unimplemented!()
        }

        fn verify_delta<Child>(
            _: &Self::Parameters,
            _: &Self::Delta,
        ) -> Result<bool, freenet_stdlib::prelude::ContractError>
        where
            Child: ComposableContract,
        {
            unimplemented!()
        }

        fn merge(
            &mut self,
            _: &Self::Parameters,
            _: &freenet_stdlib::composers::TypedUpdateData<Self>,
            _: &freenet_stdlib::prelude::RelatedContractsContainer,
        ) -> freenet_stdlib::composers::MergeResult {
            unimplemented!()
        }

        fn summarize<ParentSummary>(
            &self,
            _: &Self::Parameters,
            _: &mut ParentSummary,
        ) -> Result<(), freenet_stdlib::prelude::ContractError>
        where
            ParentSummary:
                freenet_stdlib::composers::ComposableSummary<<Self as ComposableContract>::Summary>,
        {
            unimplemented!()
        }

        fn delta(
            &self,
            _: &Self::Parameters,
            _: &Self::Summary,
        ) -> Result<Self::Delta, freenet_stdlib::prelude::ContractError> {
            unimplemented!()
        }
    }
}
