//! The `#[delegate(manifest(...))]` attribute and the stdlib's own
//! `DelegateManifest` must agree byte for byte: the macro writes JSON by hand
//! (a proc-macro crate cannot depend on the types it describes), and the node
//! reads it with `DelegateManifest::from_bytes`.

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

struct LifecycleOnly;

#[delegate(manifest(lifecycle = [NodeStarted]))]
impl DelegateInterface for LifecycleOnly {
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
        DelegateManifest::from_bytes(LifecycleOnly::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes())
            .unwrap();
    assert_eq!(
        only,
        DelegateManifest::new(vec![LifecycleKind::NodeStarted], vec![])
    );
    assert_eq!(
        LifecycleOnly::__FREENET_DELEGATE_MANIFEST_JSON.as_bytes(),
        only.to_bytes().as_slice()
    );
}
