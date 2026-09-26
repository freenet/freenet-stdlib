use freenet_stdlib::prelude::*;

pub struct Fixture;

#[delegate(manifest(
    lifecycle = [Installed, NodeStarted],
    capabilities = [Background],
    wakeups = [heartbeat = 300]
))]
impl DelegateInterface for Fixture {
    fn process(
        _ctx: &mut DelegateCtx,
        _parameters: Parameters<'static>,
        _origin: Option<MessageOrigin>,
        _message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        Ok(vec![])
    }
}
