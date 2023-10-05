use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ItemImpl, Token, Type, TypePath};

use crate::AttributeArgs;

#[derive(Debug)]
struct ChildrenPaths {
    paths: Vec<TypePath>,
}

impl syn::parse::Parse for ChildrenPaths {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut paths = vec![];
        let mut punctuated;
        let mut count = 0;
        while !input.is_empty() {
            punctuated = input.parse::<Token![,]>().ok().is_some();
            if count > 0 && !punctuated {
                return Err(syn::Error::new(
                    input.span(),
                    "arguments must be comma separated",
                ));
            }
            let path = input.parse::<TypePath>()?;
            paths.push(path);
            count += 1;
        }
        Ok(ChildrenPaths { paths })
    }
}

pub(crate) fn composable_contract_ffi_impl(
    input: &ItemImpl,
    args: &AttributeArgs,
) -> proc_macro::TokenStream {
    let type_name = match &*input.self_ty {
        Type::Path(p) => p.clone(),
        _ => panic!(),
    };

    let mut children: Vec<TypePath> = vec![];
    let mut attr_span = args.args.span();
    let mut found_children = false;
    for m in args.args.iter() {
        match m {
            syn::Meta::List(list) => {
                if !list
                    .path
                    .get_ident()
                    .map(|id| id == "children")
                    .unwrap_or(false)
                {
                    return quote_spanned! {
                        list.span() =>
                        compile_error!("only a `children` list argument allowed");
                    }
                    .into();
                } else {
                    if found_children {
                        return quote_spanned! {
                            list.span() =>
                            compile_error!("only one `children` list allowed");
                        }
                        .into();
                    }
                    found_children = true;
                    let tokens: proc_macro::TokenStream = list.tokens.clone().into();
                    let children_paths = syn::parse_macro_input!(tokens as ChildrenPaths);
                    children.extend(children_paths.paths);
                    attr_span = list.span();
                }
            }
            other => {
                return quote_spanned! {
                    other.span() =>
                    compile_error!("only list arguments allowed");
                }
                .into()
            }
        }
    }

    if children.is_empty() {
        return quote_spanned! {
            attr_span =>
            compile_error!("at least one ComposableContract child is required");
        }
        .into();
    }

    let s = ImplStruct {
        type_name,
        children,
    };

    let ffi = s.gen_extern_functions();
    let contract_iface = s.gen_contract_iface();
    quote! {
        #input
        #contract_iface
        #ffi
    }
    .into()
}

pub(crate) fn raw_contract_ffi_impl(input: &ItemImpl) -> proc_macro::TokenStream {
    let type_name = match &*input.self_ty {
        Type::Path(p) => p.clone(),
        _ => panic!(),
    };
    let s = ImplStruct {
        type_name,
        children: vec![],
    };
    let ffi = s.gen_extern_functions();
    quote! {
        #input
        #ffi
    }
    .into()
}

struct ImplStruct {
    type_name: TypePath,
    children: Vec<TypePath>,
}

impl ImplStruct {
    fn ffi_ret_type(&self) -> TokenStream {
        quote!(i64)
    }

    fn gen_contract_iface(&self) -> TokenStream {
        let type_name = &self.type_name;

        let validate_state_impl = self.children.iter().map(|child| {
            quote! {
                match ::freenet_stdlib::composers::from_bytes::inner_validate_state::<
                    #type_name,
                    #child,
                    <#type_name as ::freenet_stdlib::composers::ComposableContract>::Context,
                >(parameters.clone(), state.clone(), related.clone())? {
                    ::freenet_stdlib::prelude::ValidateResult::Valid => {}
                    ::freenet_stdlib::prelude::ValidateResult::Invalid => {
                        return ::core::result::Result::Ok(::freenet_stdlib::prelude::ValidateResult::Invalid)
                    }
                    ::freenet_stdlib::prelude::ValidateResult::RequestRelated(req) => {
                        return ::core::result::Result::Ok(::freenet_stdlib::prelude::ValidateResult::RequestRelated(req))
                    }
                }
            }
        });

        let validate_delta_impl = self.children.iter().map(|child| {
            quote! {
                if !::freenet_stdlib::composers::from_bytes::inner_validate_delta::<
                    #type_name,
                    #child,
                >(parameters.clone(), delta.clone())? {
                    return ::core::result::Result::Ok(false);
                }
            }
        });

        let update_state_impl = self.children.iter().map(|child| {
            quote! {{
                let modification = ::freenet_stdlib::composers::from_bytes::inner_update_state::<
                    #type_name,
                    #child,
                >(parameters.clone(), final_update.clone(), data.clone())?;
                if modification.requires_dependencies() {
                    return ::core::result::Result::Ok(modification);
                } else  {
                    final_update = modification.unwrap_valid();
                }
            }}
        });

        quote! {
            impl ::freenet_stdlib::prelude::ContractInterface for #type_name {
                fn validate_state(
                    parameters: ::freenet_stdlib::prelude::Parameters<'static>,
                    state: ::freenet_stdlib::prelude::State<'static>,
                    related: ::freenet_stdlib::prelude::RelatedContracts<'static>,
                ) -> ::core::result::Result<
                    ::freenet_stdlib::prelude::ValidateResult,
                    ::freenet_stdlib::prelude::ContractError,
                > {
                    #(#validate_state_impl)*
                    ::core::result::Result::Ok(::freenet_stdlib::prelude::ValidateResult::Valid)
                }

                fn validate_delta(
                    parameters: ::freenet_stdlib::prelude::Parameters<'static>,
                    delta: ::freenet_stdlib::prelude::StateDelta<'static>,
                ) -> ::core::result::Result<bool, ::freenet_stdlib::prelude::ContractError> {
                    #(#validate_delta_impl)*
                    ::core::result::Result::Ok(true)
                }

                fn update_state(
                    parameters: ::freenet_stdlib::prelude::Parameters<'static>,
                    state: ::freenet_stdlib::prelude::State<'static>,
                    data: Vec<freenet_stdlib::prelude::UpdateData<'static>>,
                ) -> ::core::result::Result<
                    ::freenet_stdlib::prelude::UpdateModification<'static>,
                    ::freenet_stdlib::prelude::ContractError,
                > {
                    let mut final_update = state;
                    #(#update_state_impl)*
                    Ok(::freenet_stdlib::prelude::UpdateModification::valid(final_update))
                }

                fn summarize_state(
                    parameters: ::freenet_stdlib::prelude::Parameters<'static>,
                    state: ::freenet_stdlib::prelude::State<'static>,
                ) -> ::core::result::Result<
                    ::freenet_stdlib::prelude::StateSummary<'static>,
                    ::freenet_stdlib::prelude::ContractError,
                > {
                    let mut summary: ::core::option::Option<<#type_name as ::freenet_stdlib::composers::ComposableContract>::Summary> = ::core::option::Option::None;
                    let summary = ::freenet_stdlib::composers::from_bytes::inner_summarize_state::<
                        #type_name,
                    >(parameters.clone(), state.clone())?;
                    let serializable_summary = <#type_name as ::freenet_stdlib::prelude::SerializationAdapter>::Summary::from(summary);
                    let encoded_summary = ::freenet_stdlib::prelude::Encoder::serialize(&serializable_summary)?;
                    Ok(encoded_summary.into())
                }

                fn get_state_delta(
                    parameters: ::freenet_stdlib::prelude::Parameters<'static>,
                    state: ::freenet_stdlib::prelude::State<'static>,
                    summary: ::freenet_stdlib::prelude::StateSummary<'static>,
                ) -> ::core::result::Result<
                    ::freenet_stdlib::prelude::StateDelta<'static>,
                    ::freenet_stdlib::prelude::ContractError,
                > {
                    let delta = ::freenet_stdlib::composers::from_bytes::inner_state_delta::<
                        #type_name,
                    >(parameters.clone(), state.clone(), summary.clone())?;
                    let serializable_delta = <#type_name as SerializationAdapter>::Delta::from(delta);
                    let encoded_delta = ::freenet_stdlib::prelude::Encoder::serialize(&serializable_delta)?;
                    Ok(encoded_delta.into())
                }
            }
        }
    }

    fn gen_extern_functions(&self) -> TokenStream {
        let validate_state_fn = self.gen_validate_state_fn();
        let validate_delta_fn = self.gen_validate_delta_fn();
        let update_fn = self.gen_update_state_fn();
        let summarize_fn = self.gen_summarize_state_fn();
        let get_delta_fn = self.gen_get_state_delta();
        quote! {
            #validate_state_fn
            #validate_delta_fn
            #update_fn
            #summarize_fn
            #get_delta_fn
        }
    }

    fn gen_validate_state_fn(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-contract")]
            pub extern "C" fn validate_state(parameters: i64, state: i64, related: i64) -> #ret {
                ::freenet_stdlib::memory::wasm_interface::inner_validate_state::<#type_name>(parameters, state, related)
            }
        }
    }

    fn gen_validate_delta_fn(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-contract")]
            pub extern "C" fn validate_delta(parameters: i64, delta: i64) -> #ret {
                ::freenet_stdlib::memory::wasm_interface::inner_validate_delta::<#type_name>(parameters, delta)
            }
        }
    }

    fn gen_update_state_fn(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-contract")]
            pub extern "C" fn update_state(parameters: i64, state: i64, delta: i64) -> #ret {
                ::freenet_stdlib::memory::wasm_interface::inner_update_state::<#type_name>(parameters, state, delta)
            }
        }
    }

    fn gen_summarize_state_fn(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-contract")]
            pub extern "C" fn summarize_state(parameters: i64, state: i64) -> #ret {
                ::freenet_stdlib::memory::wasm_interface::inner_summarize_state::<#type_name>(parameters, state)
            }
        }
    }

    fn gen_get_state_delta(&self) -> TokenStream {
        let type_name = &self.type_name;
        let ret = self.ffi_ret_type();
        quote! {
            #[no_mangle]
            #[cfg(feature = "freenet-main-contract")]
            pub extern "C" fn get_state_delta(parameters: i64, state: i64, summary: i64) -> #ret {
                ::freenet_stdlib::memory::wasm_interface::inner_get_state_delta::<#type_name>(parameters, state, summary)
            }
        }
    }
}
