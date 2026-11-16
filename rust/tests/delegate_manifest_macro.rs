//! The `#[delegate(manifest(...))]` attribute and the stdlib's own
//! `DelegateManifest` must agree byte for byte: the macro writes JSON by hand
//! (a proc-macro crate cannot depend on the types it describes), and the node
//! reads it with `DelegateManifest::from_bytes`.

// `#[delegate]` emits `cfg(feature = "freenet-main-delegate")`, a feature a
// delegate crate defines and this test crate does not.
#![allow(unexpected_cfgs)]

use freenet_stdlib::prelude::*;

struct WithManifest;

#[delegate(manifest(lifecycle = [Installed, NodeStarted], capabilities = [Background]))]
impl DelegateInterface for WithManifest {
    fn process(
        _ctx: &mut DelegateCtx,
        _parameters: Parameters<'static>,
        _origin: Option<MessageOrigin>,
        _message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        Ok(vec![])
    }
}

struct WithWakeups;

#[delegate(manifest(
    lifecycle = [NodeStarted],
    capabilities = [Background],
    wakeups = [heartbeat = 300, renew = 86400],
))]
impl DelegateInterface for WithWakeups {
    fn process(
        _ctx: &mut DelegateCtx,
        _parameters: Parameters<'static>,
        _origin: Option<MessageOrigin>,
        _message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        Ok(vec![])
    }
}

/// The macro writes the wake-up list by hand; the stdlib must read it back
/// as exactly the schedules declared, and serialize them to the same bytes.
#[test]
fn macro_wakeups_match_the_stdlib_serializer() {
    let expected = DelegateManifest::new(
        vec![LifecycleKind::NodeStarted],
        vec![Capability::Background],
    )
    .with_wakeup("heartbeat", 300)
    .with_wakeup("renew", 86400);
    let json = WithWakeups::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes();
    assert_eq!(json, expected.to_bytes().as_slice());
    let read = DelegateManifest::from_bytes(json).unwrap();
    assert_eq!(read, expected);
    assert_eq!(
        read.effective_wakeups(),
        vec![
            (b"heartbeat".to_vec(), std::time::Duration::from_secs(300)),
            (b"renew".to_vec(), std::time::Duration::from_secs(86400)),
        ]
    );
}

struct BackgroundOnly;

#[delegate(manifest(capabilities = [Background]))]
impl DelegateInterface for BackgroundOnly {
    fn process(
        _ctx: &mut DelegateCtx,
        _parameters: Parameters<'static>,
        _origin: Option<MessageOrigin>,
        _message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        Ok(vec![])
    }
}

#[test]
fn macro_json_matches_the_stdlib_serializer() {
    let expected = DelegateManifest::new(
        vec![LifecycleKind::Installed, LifecycleKind::NodeStarted],
        vec![Capability::Background],
    );
    assert_eq!(
        WithManifest::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes(),
        expected.to_bytes().as_slice()
    );
    assert_eq!(
        DelegateManifest::from_bytes(WithManifest::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes())
            .unwrap(),
        expected
    );

    let only =
        DelegateManifest::from_bytes(BackgroundOnly::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes())
            .unwrap();
    assert_eq!(
        only,
        DelegateManifest::new(vec![], vec![Capability::Background])
    );
    assert_eq!(
        BackgroundOnly::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes(),
        only.to_bytes().as_slice()
    );
}
