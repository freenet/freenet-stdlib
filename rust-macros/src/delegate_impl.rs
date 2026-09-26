use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ItemImpl, Meta, Token, Type, TypePath};

/// Must match `freenet_stdlib::prelude::MANIFEST_SECTION_NAME`. Checked at
/// compile time in every delegate that declares a manifest (see
/// `manifest_section`).
const MANIFEST_SECTION_NAME: &str = "freenet-manifest";
/// Must match `freenet_stdlib::prelude::MANIFEST_VERSION`; checked the same way.
const MANIFEST_VERSION: u16 = 1;

/// `(source name, JSON name)` for each lifecycle kind this macro accepts.
/// The source name must be a `freenet_stdlib::prelude::LifecycleKind`
/// variant (the generated code names it, so a stdlib without the variant
/// fails to compile) and the JSON name its serde name (pinned by the stdlib
/// test `macro_json_matches_the_stdlib_serializer`).
const LIFECYCLE_KINDS: &[(&str, &str)] =
    &[("Installed", "installed"), ("NodeStarted", "node_started")];
/// Same, for `freenet_stdlib::prelude::Capability`.
const CAPABILITIES: &[(&str, &str)] = &[("Background", "background")];

/// Mirror `freenet_stdlib::prelude::{MIN_WAKEUP_INTERVAL_SECS,
/// MAX_WAKEUP_INTERVAL_SECS, MAX_WAKEUP_TAG_BYTES, MAX_WAKEUPS}`, for readable
/// errors. The generated code also asserts each entry against the stdlib's own
/// constants. That catches this macro being LOOSER than its stdlib (it would
/// accept something the stdlib's bounds reject); a macro stricter than its
/// stdlib only refuses more, which is harmless. Against a stdlib without
/// wake-ups (< 0.12.1) the generated code fails to compile because the
/// constants do not exist, which is the intended outcome with a less friendly
/// message.
const MIN_WAKEUP_INTERVAL_SECS: u64 = 60;
const MAX_WAKEUP_INTERVAL_SECS: u64 = 7 * 24 * 3600;
const MAX_WAKEUP_TAG_BYTES: usize = 64;
const MAX_WAKEUPS: usize = 4;

/// A parsed `manifest(...)` argument.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ManifestArgs {
    /// `(source name, JSON name)`, deduplicated, in declaration order.
    pub lifecycle: Vec<(&'static str, &'static str)>,
    pub capabilities: Vec<(&'static str, &'static str)>,
    /// `(tag, every_secs)`, in declaration order, tags unique.
    pub wakeups: Vec<(String, u64)>,
}

impl ManifestArgs {
    /// The section payload. Byte-identical to what
    /// `DelegateManifest::to_bytes` produces for the same manifest; the
    /// stdlib test `macro_json_matches_the_stdlib_serializer` pins that.
    pub fn to_json(&self) -> String {
        fn list(items: &[(&str, &str)]) -> String {
            items
                .iter()
                .map(|(_, s)| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",")
        }
        // `wakeups` is omitted when empty, exactly as the stdlib serializer
        // does, so a manifest without wake-ups is byte-identical to the one
        // stdlib 0.12.0 wrote (the manifest section does not change on upgrade).
        let wakeups = if self.wakeups.is_empty() {
            String::new()
        } else {
            // Tags are Rust identifiers, so they need no JSON escaping.
            let entries = self
                .wakeups
                .iter()
                .map(|(tag, secs)| format!("{{\"tag\":\"{tag}\",\"every_secs\":{secs}}}"))
                .collect::<Vec<_>>()
                .join(",");
            format!(",\"wakeups\":[{entries}]")
        };
        format!(
            "{{\"manifest_version\":{MANIFEST_VERSION},\"lifecycle\":[{}],\"capabilities\":[{}]{wakeups}}}",
            list(&self.lifecycle),
            list(&self.capabilities)
        )
    }
}

/// Parse the `#[delegate(...)]` arguments. `Ok(None)` when there is no
/// `manifest(...)`. Any other argument is an error: the attribute took no
/// arguments before manifests, and a misspelt `manifest` silently producing a
/// delegate with no manifest would be the worst outcome.
pub fn parse_manifest_args(
    args: &Punctuated<Meta, Token![,]>,
) -> syn::Result<Option<ManifestArgs>> {
    let mut manifest = None;
    for meta in args {
        let Meta::List(list) = meta else {
            return Err(syn::Error::new(
                meta.span(),
                "unsupported #[delegate] argument; expected `manifest(...)`",
            ));
        };
        if !list.path.is_ident("manifest") {
            return Err(syn::Error::new(
                list.path.span(),
                "unsupported #[delegate] argument; expected `manifest(...)`",
            ));
        }
        if manifest.is_some() {
            return Err(syn::Error::new(
                list.span(),
                "`manifest` given more than once",
            ));
        }
        let entries =
            list.parse_args_with(Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated)?;
        let mut m = ManifestArgs::default();
        let (mut seen_lifecycle, mut seen_caps, mut seen_wakeups) = (false, false, false);
        for entry in entries {
            if entry.path.is_ident("wakeups") {
                if seen_wakeups {
                    return Err(syn::Error::new(
                        entry.path.span(),
                        "manifest key given more than once",
                    ));
                }
                seen_wakeups = true;
                m.wakeups = parse_wakeups(&entry.value)?;
                continue;
            }
            let (table, out, seen, what) = if entry.path.is_ident("lifecycle") {
                (
                    LIFECYCLE_KINDS,
                    &mut m.lifecycle,
                    &mut seen_lifecycle,
                    "lifecycle kind",
                )
            } else if entry.path.is_ident("capabilities") {
                (
                    CAPABILITIES,
                    &mut m.capabilities,
                    &mut seen_caps,
                    "capability",
                )
            } else {
                return Err(syn::Error::new(
                    entry.path.span(),
                    "unknown manifest key; expected `lifecycle`, `capabilities` or `wakeups`",
                ));
            };
            if *seen {
                return Err(syn::Error::new(
                    entry.path.span(),
                    "manifest key given more than once",
                ));
            }
            *seen = true;
            let Expr::Array(array) = &entry.value else {
                return Err(syn::Error::new(
                    entry.value.span(),
                    "expected a list, e.g. `[Installed, NodeStarted]`",
                ));
            };
            for elem in &array.elems {
                let name = match elem {
                    Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()),
                    _ => None,
                };
                let entry = name
                    .as_deref()
                    .and_then(|n| table.iter().find(|(src, _)| *src == n))
                    .copied()
                    .ok_or_else(|| {
                        let known: Vec<_> = table.iter().map(|(s, _)| *s).collect();
                        syn::Error::new(
                            elem.span(),
                            format!("unknown {what}; known: {}", known.join(", ")),
                        )
                    })?;
                if !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
        // A lifecycle event is a run with no app open, which is exactly what
        // `Background` grants. Without it the node never delivers the event,
        // so a manifest listing one without the other is a silent no-op.
        if !m.lifecycle.is_empty() && !m.capabilities.iter().any(|(s, _)| *s == "Background") {
            return Err(syn::Error::new(
                list.span(),
                "lifecycle events are only delivered to a delegate whose app holds the \
                 Background grant; add `capabilities = [Background]`",
            ));
        }
        // Same for wake-ups: they are runs with no app open.
        if !m.wakeups.is_empty() && !m.capabilities.iter().any(|(s, _)| *s == "Background") {
            return Err(syn::Error::new(
                list.span(),
                "wake-ups are only delivered to a delegate whose app holds the \
                 Background grant; add `capabilities = [Background]`",
            ));
        }
        manifest = Some(m);
    }
    Ok(manifest)
}

/// Parse `wakeups = [tag = seconds, ...]`.
///
/// Stricter than the node, on purpose: the node clamps an out-of-range
/// interval and drops a bad entry (it must read manifests written by any
/// tool), while the macro refuses them, because a delegate author who wrote
/// `heartbeat = 10` wants to know it will not run every ten seconds.
fn parse_wakeups(value: &Expr) -> syn::Result<Vec<(String, u64)>> {
    let Expr::Array(array) = value else {
        return Err(syn::Error::new(
            value.span(),
            "expected a list, e.g. `[heartbeat = 300]` (tag = interval in seconds)",
        ));
    };
    let mut out: Vec<(String, u64)> = Vec::new();
    for elem in &array.elems {
        let Expr::Assign(assign) = elem else {
            return Err(syn::Error::new(
                elem.span(),
                "expected `tag = seconds`, e.g. `heartbeat = 300`",
            ));
        };
        // `unraw`: `r#loop = 60` means the tag "loop", not "r#loop".
        let tag = match &*assign.left {
            Expr::Path(p) => p.path.get_ident().map(|i| i.unraw().to_string()),
            _ => None,
        }
        .ok_or_else(|| {
            syn::Error::new(
                assign.left.span(),
                "a wake-up tag must be a plain identifier, e.g. `heartbeat`",
            )
        })?;
        let secs = match &*assign.right {
            Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(i),
                ..
            }) => i.base10_parse::<u64>()?,
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "a wake-up interval must be an integer number of seconds",
                ))
            }
        };
        if tag.len() > MAX_WAKEUP_TAG_BYTES {
            return Err(syn::Error::new(
                assign.left.span(),
                format!("wake-up tag is longer than {MAX_WAKEUP_TAG_BYTES} bytes"),
            ));
        }
        if !(MIN_WAKEUP_INTERVAL_SECS..=MAX_WAKEUP_INTERVAL_SECS).contains(&secs) {
            return Err(syn::Error::new(
                assign.right.span(),
                format!(
                    "wake-up interval must be between {MIN_WAKEUP_INTERVAL_SECS} and \
                     {MAX_WAKEUP_INTERVAL_SECS} seconds"
                ),
            ));
        }
        if out.iter().any(|(t, _)| *t == tag) {
            return Err(syn::Error::new(
                assign.left.span(),
                "wake-up tag given more than once",
            ));
        }
        out.push((tag, secs));
    }
    if out.len() > MAX_WAKEUPS {
        return Err(syn::Error::new(
            array.span(),
            format!("at most {MAX_WAKEUPS} wake-ups per delegate"),
        ));
    }
    Ok(out)
}

/// The custom section carrying the manifest, plus a hidden associated const
/// holding the same JSON so it can be inspected in native tests (the section
/// itself only exists in a WASM build).
pub fn manifest_section(item: &ItemImpl, manifest: &ManifestArgs) -> TokenStream {
    let type_name = &item.self_ty;
    let json = manifest.to_json();
    let bytes = json.as_bytes();
    let len = bytes.len();
    let byte_lits = bytes.iter().map(|b| quote!(#b));
    let section = syn::LitStr::new(MANIFEST_SECTION_NAME, Span::call_site());
    // Name every listed kind and capability through the stdlib, so a delegate
    // whose stdlib lacks one fails to compile instead of advertising an event
    // its `InboundDelegateMsg` cannot decode. (The macros crate can be newer
    // than the stdlib it is paired with.)
    let kind_refs = manifest.lifecycle.iter().map(|(src, _)| {
        let v = syn::Ident::new(src, Span::call_site());
        quote!(const _: ::freenet_stdlib::prelude::LifecycleKind = ::freenet_stdlib::prelude::LifecycleKind::#v;)
    });
    let cap_refs = manifest.capabilities.iter().map(|(src, _)| {
        let v = syn::Ident::new(src, Span::call_site());
        quote!(const _: ::freenet_stdlib::prelude::Capability = ::freenet_stdlib::prelude::Capability::#v;)
    });
    // Check each wake-up against the stdlib's OWN bounds too, so a macro and
    // stdlib that disagree fail to compile instead of emitting a manifest the
    // node reads differently. (This also requires a stdlib that knows
    // wake-ups, i.e. >= 0.12.1.)
    let wakeup_count = manifest.wakeups.len();
    let wakeup_checks = manifest.wakeups.iter().map(|(tag, secs)| {
        let tag_len = tag.len();
        quote! {
            const _: () = ::core::assert!(
                #secs >= ::freenet_stdlib::prelude::MIN_WAKEUP_INTERVAL_SECS
                    && #secs <= ::freenet_stdlib::prelude::MAX_WAKEUP_INTERVAL_SECS
                    && #tag_len <= ::freenet_stdlib::prelude::MAX_WAKEUP_TAG_BYTES,
                "a wake-up is outside the bounds of this freenet-stdlib; use matching freenet-macros and freenet-stdlib versions"
            );
        }
    });
    let wakeup_count_check = (wakeup_count > 0).then(|| {
        quote! {
            const _: () = ::core::assert!(
                #wakeup_count <= ::freenet_stdlib::prelude::MAX_WAKEUPS,
                "more wake-ups than this freenet-stdlib allows"
            );
        }
    });
    let version = MANIFEST_VERSION;
    quote! {
        #(#kind_refs)*
        #(#cap_refs)*
        #(#wakeup_checks)*
        #wakeup_count_check
        const _: () = ::core::assert!(
            ::freenet_stdlib::prelude::__manifest_macro_agrees(#section, #version),
            "freenet-macros and freenet-stdlib disagree on the delegate manifest section; use matching versions"
        );

        // One manifest per WASM module: a second `manifest(...)` in the same
        // crate is a duplicate definition of this static.
        // WASM-only: on other targets a custom link section is either
        // meaningless or, on Mach-O, a hard error about the section name.
        #[cfg(all(feature = "freenet-main-delegate", target_family = "wasm"))]
        #[doc(hidden)]
        #[used]
        #[link_section = #section]
        pub static __FREENET_DELEGATE_MANIFEST: [u8; #len] = [#(#byte_lits),*];

        impl #type_name {
            /// The manifest JSON embedded in this delegate's WASM.
            #[doc(hidden)]
            pub const __FREENET_DELEGATE_MANIFEST_JSON: &'static str = #json;
        }
    }
}

pub fn ffi_impl_wrap(item: &ItemImpl) -> TokenStream {
    let type_name = match &*item.self_ty {
        Type::Path(p) => p.clone(),
        _ => panic!(),
    };
    let s = ImplStruct { type_name };
    let process_fn = s.gen_process_fn();
    quote!(#process_fn)
}

struct ImplStruct {
    type_name: TypePath,
}

impl ImplStruct {
    fn ffi_ret_type(&self) -> TokenStream {
        quote!(i64)
    }

    fn gen_process_fn(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        let set_logger = crate::common::set_logger();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-delegate")]
            pub extern "C" fn process(parameters: i64, origin: i64, inbound: i64) -> #ret {
                #set_logger
                let parameters = unsafe {
                    let param_buf = &*(parameters as *const ::freenet_stdlib::memory::buf::BufferBuilder);
                    let bytes = &*std::ptr::slice_from_raw_parts(
                        param_buf.start(),
                        param_buf.bytes_written(),
                    );
                    Parameters::from(bytes)
                };
                let origin: Option<::freenet_stdlib::prelude::MessageOrigin> = unsafe {
                    let origin_buf = &*(origin as *const ::freenet_stdlib::memory::buf::BufferBuilder);
                    let bytes = &*std::ptr::slice_from_raw_parts(
                        origin_buf.start(),
                        origin_buf.bytes_written(),
                    );
                    if bytes.is_empty() {
                        None
                    } else {
                        match ::freenet_stdlib::prelude::bincode::deserialize(bytes) {
                            Ok(v) => Some(v),
                            Err(_) => None,
                        }
                    }
                };
                let inbound = unsafe {
                    let inbound_buf = &mut *(inbound as *mut ::freenet_stdlib::memory::buf::BufferBuilder);
                    let bytes =
                        &*std::ptr::slice_from_raw_parts(inbound_buf.start(), inbound_buf.bytes_written());
                    match ::freenet_stdlib::prelude::bincode::deserialize(bytes) {
                        Ok(v) => v,
                        Err(err) => return ::freenet_stdlib::prelude::DelegateInterfaceResult::from(
                            Err::<::std::vec::Vec<::freenet_stdlib::prelude::OutboundDelegateMsg>, _>(::freenet_stdlib::prelude::DelegateError::Deser(format!("{}", err)))
                        ).into_raw(),
                    }
                };

                // Create opaque handle for context access (includes secrets).
                // SAFETY: The runtime has set up the delegate execution environment
                // before calling this function, so the host functions are available.
                let mut ctx = unsafe { ::freenet_stdlib::prelude::DelegateCtx::__new() };

                let result = <#type_name as ::freenet_stdlib::prelude::DelegateInterface>::process(
                    &mut ctx,
                    parameters,
                    origin,
                    inbound
                );
                ::freenet_stdlib::prelude::DelegateInterfaceResult::from(result).into_raw()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(tokens: &str) -> syn::Result<Option<ManifestArgs>> {
        let args =
            syn::parse::Parser::parse_str(Punctuated::<Meta, Token![,]>::parse_terminated, tokens)?;
        parse_manifest_args(&args)
    }

    #[test]
    fn no_arguments_means_no_manifest() {
        assert_eq!(parse("").unwrap(), None);
    }

    #[test]
    fn parses_and_dedupes() {
        let m = parse("manifest(lifecycle = [NodeStarted, Installed, NodeStarted], capabilities = [Background])")
            .unwrap()
            .unwrap();
        assert_eq!(
            m.lifecycle,
            vec![("NodeStarted", "node_started"), ("Installed", "installed")]
        );
        assert_eq!(m.capabilities, vec![("Background", "background")]);
        assert_eq!(
            m.to_json(),
            r#"{"manifest_version":1,"lifecycle":["node_started","installed"],"capabilities":["background"]}"#
        );
    }

    #[test]
    fn an_empty_manifest_is_allowed() {
        let m = parse("manifest()").unwrap().unwrap();
        assert_eq!(
            m.to_json(),
            r#"{"manifest_version":1,"lifecycle":[],"capabilities":[]}"#
        );
    }

    #[test]
    fn rejects_mistakes_instead_of_dropping_them() {
        for bad in [
            "manfest(lifecycle = [Installed])",
            "manifest(lifecycle = [Instaled])",
            "manifest(capabilities = [Teleport])",
            "manifest(lifecycles = [Installed])",
            "manifest(lifecycle = Installed)",
            "manifest(lifecycle = [Installed], lifecycle = [NodeStarted])",
            "manifest(), manifest()",
            "something_else",
        ] {
            assert!(parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn parses_wakeups_and_appends_them_to_the_json() {
        let m = parse(
            "manifest(lifecycle = [NodeStarted], capabilities = [Background], \
             wakeups = [heartbeat = 300, renew = 86400])",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            m.wakeups,
            vec![("heartbeat".to_string(), 300), ("renew".to_string(), 86400)]
        );
        assert_eq!(
            m.to_json(),
            r#"{"manifest_version":1,"lifecycle":["node_started"],"capabilities":["background"],"wakeups":[{"tag":"heartbeat","every_secs":300},{"tag":"renew","every_secs":86400}]}"#
        );
        // An empty list writes nothing, like the stdlib serializer.
        let m = parse("manifest(capabilities = [Background], wakeups = [])")
            .unwrap()
            .unwrap();
        assert_eq!(
            m.to_json(),
            r#"{"manifest_version":1,"lifecycle":[],"capabilities":["background"]}"#
        );
    }

    #[test]
    fn wakeup_bounds_are_enforced_at_their_boundaries() {
        let ok = |w: &str| {
            parse(&format!(
                "manifest(capabilities = [Background], wakeups = [{w}])"
            ))
        };
        assert!(ok("a = 59").is_err());
        assert!(ok("a = 60").is_ok());
        assert!(ok("a = 604800").is_ok());
        assert!(ok("a = 604801").is_err());
        assert!(ok(&format!("{} = 60", "t".repeat(MAX_WAKEUP_TAG_BYTES))).is_ok());
        assert!(ok(&format!("{} = 60", "t".repeat(MAX_WAKEUP_TAG_BYTES + 1))).is_err());
        assert!(ok("a = 60, b = 60, c = 60, d = 60").is_ok());
        assert!(ok("a = 60, b = 60, c = 60, d = 60, e = 60").is_err());
        assert!(ok("a = 60, a = 120").is_err());
        for bad in [
            "a",
            "\"a\" = 60",
            "a = \"60\"",
            "a = 60.0",
            "a::b = 60",
            "a = -1",
        ] {
            assert!(ok(bad).is_err(), "{bad} should be rejected");
        }
        assert!(parse("manifest(capabilities = [Background], wakeups = a)").is_err());
        // A raw identifier is the plain name.
        let m = ok("r#loop = 60").unwrap().unwrap();
        assert_eq!(m.wakeups, vec![("loop".to_string(), 60)]);
        assert!(parse(
            "manifest(capabilities = [Background], wakeups = [a = 60], wakeups = [b = 60])"
        )
        .is_err());
    }

    #[test]
    fn wakeups_require_background() {
        let err = parse("manifest(wakeups = [heartbeat = 300])").unwrap_err();
        assert!(err.to_string().contains("Background"), "{err}");
    }

    #[test]
    fn lifecycle_requires_background() {
        let err = parse("manifest(lifecycle = [Installed])").unwrap_err();
        assert!(err.to_string().contains("Background"), "{err}");
        assert!(parse("manifest(lifecycle = [Installed], capabilities = [Background])").is_ok());
        // Background alone is fine: later capabilities build on it.
        assert!(parse("manifest(capabilities = [Background])").is_ok());
    }
}
