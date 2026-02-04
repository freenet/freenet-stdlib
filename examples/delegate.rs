//! This contract just checks that macros compile etc.
// ANCHOR: delegateifce
use freenet_stdlib::prelude::*;

pub const RANDOM_SIGNATURE: &[u8] = &[6, 8, 2, 5, 6, 9, 9, 10];

struct Delegate;

#[delegate]
impl DelegateInterface for Delegate {
    fn process(
        ctx: &mut DelegateCtx,
        secrets: &mut SecretsStore,
        _parameters: Parameters<'static>,
        _attested: Option<&'static [u8]>,
        _messages: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        // Example: read context
        let _data = ctx.read();

        // Example: write context
        ctx.write(b"some state");

        // Example: access secrets synchronously
        if secrets.has(b"my_key") {
            let _secret = secrets.get(b"my_key");
        }

        // Example: store a secret
        secrets.set(b"new_key", b"secret_value");

        unimplemented!()
    }
}
// ANCHOR_END: delegateifce

fn main() {}
