/*
mod example_2 {
    use crate::parameters;

    use super::*;

    pub struct PublicKey {}
    pub struct Signature {}

    impl<T: ComposableContract> ComposableContract for Vec<T>
    where
        T: ComposableContract,
    {
        type Context = T;
        type Parameters = T::Parameters;
        type Summary = T::Summary;
        type Delta = T::Delta;

        fn verify(
            &self,
            parameters: &Self::Parameters,
            context: &Self::Context,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn Error>> {
            unimplemented!()
        }
    }

    pub struct ChatRoomParameters {
        pub owner_public_key: PublicKey,
    }

    impl ComposableParameters for ChatRoomParameters {}

    /// Contract 0
    pub struct ChatRoom {
        pub members: Vec<ChatRoomMember>,
        pub messages: Vec<SignedSyncable<ChatRoomMessage>>,
    }

    impl ComposableContract for ChatRoom {
        type Context = ();
        type Parameters = ChatRoomParameters;
        type Summary = ();
        type Delta = ();

        fn verify(
            &self,
            parameters: &Self::Parameters,
            context: &Self::Context,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn Error>> {
            // automatically generated
            {
                self.members.verify(&parameters, self, related)?;
            }
            {
                self.messages.verify(&parameters, self, related)?;
            }
            Ok(())
        }
    }

    pub struct ChatRoomMember {
        pub name: String,
        pub public_key: PublicKey,
    }

    impl ComposableContract for ChatRoomMember {
        type Context = ();
        type Parameters = ();
        type Summary = ();
        type Delta = ();
    }

    pub struct ChatRoomMessage {
        pub message: String,
        pub author: String,
        pub signature: String,
    }

    impl ComposableContract for ChatRoomMessage {
        type Context = ();
        type Parameters = ();
        type Summary = ();
        type Delta = ();
    }

    pub struct SignedSyncable<S> {
        pub value: S,
        pub signature: Signature,
        // extractor: EXTRACTOR,
    }

    impl<S: ComposableContract> SignedSyncable<S> {}

    trait ParametersWithPublicKey: ComposableParameters {
        fn public_key(&self) -> PublicKey;
    }

    // manually implemented
    impl<S: ComposableContract> ComposableContract for SignedSyncable<S> {
        type Context = ();
        // type Parameters<T: ParametersWithPublicKey>;
        type Summary = ();
        type Delta = ();

        fn verify(
            &self,
            parameters: &Self::Parameters,
            context: &Self::Context,
            related: &RelatedContractsContainer,
        ) -> Result<(), Box<dyn Error>> {
            // Extracts a public key either from the context or the parameters using a
            // function passed as a generic type or associated type
            let public_key = parameters.public_key();
            context.extract::<PublicKey>();
            todo!()
        }

        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            delta: &Self::Delta,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            unimplemented!()
        }
    }
}
*/
