use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ItemImpl, Meta, Token, Type, TypePath};

/// Must match `freenet_stdlib::prelude::MANIFEST_SECTION_NAME`.
const MANIFEST_SECTION_NAME: &str = "freenet-manifest";
/// Must match `freenet_stdlib::prelude::MANIFEST_VERSION`.
const MANIFEST_VERSION: u16 = 1;

/// `(source name, JSON name)` for each lifecycle kind this macro accepts.
/// Must match the serde names of `freenet_stdlib::prelude::LifecycleKind`.
const LIFECYCLE_KINDS: &[(&str, &str)] =
    &[("Installed", "installed"), ("NodeStarted", "node_started")];
/// Same, for `freenet_stdlib::prelude::Capability`.
const CAPABILITIES: &[(&str, &str)] = &[("Background", "background")];

/// A parsed `manifest(...)` argument.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ManifestArgs {
    /// JSON names, deduplicated, in declaration order.
    pub lifecycle: Vec<&'static str>,
    pub capabilities: Vec<&'static str>,
}

impl ManifestArgs {
    /// The section payload. Byte-identical to what
    /// `DelegateManifest::to_bytes` produces for the same manifest; the
    /// stdlib test `macro_json_matches_the_stdlib_serializer` pins that.
    pub fn to_json(&self) -> String {
        fn list(items: &[&str]) -> String {
            items
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",")
        }
        format!(
            "{{\"manifest_version\":{MANIFEST_VERSION},\"lifecycle\":[{}],\"capabilities\":[{}]}}",
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
        let (mut seen_lifecycle, mut seen_caps) = (false, false);
        for entry in entries {
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
                    "unknown manifest key; expected `lifecycle` or `capabilities`",
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
                let json = name
                    .as_deref()
                    .and_then(|n| table.iter().find(|(src, _)| *src == n))
                    .map(|(_, json)| *json)
                    .ok_or_else(|| {
                        let known: Vec<_> = table.iter().map(|(s, _)| *s).collect();
                        syn::Error::new(
                            elem.span(),
                            format!("unknown {what}; known: {}", known.join(", ")),
                        )
                    })?;
                if !out.contains(&json) {
                    out.push(json);
                }
            }
        }
        manifest = Some(m);
    }
    Ok(manifest)
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
    quote! {
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
        assert_eq!(m.lifecycle, vec!["node_started", "installed"]);
        assert_eq!(m.capabilities, vec!["background"]);
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
}
