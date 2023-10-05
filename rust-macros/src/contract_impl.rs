use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ImplItem, ItemImpl, MetaNameValue, Token, Type, TypePath};

use crate::{AttributeArgs, ContractType};

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

struct AsocTypes {
    params: Type,
    delta: Type,
    summary: Type,
}

pub(crate) fn contract_ffi_impl(
    input: &ItemImpl,
    args: &AttributeArgs,
    c_type: ContractType,
) -> proc_macro::TokenStream {
    let attr_span = args.args.span();
    let type_name = match &*input.self_ty {
        Type::Path(p) => p.clone(),
        _ => panic!(),
    };
    let mut impl_trait = ImplTrait {
        type_name,
        children: vec![],
    };

    match c_type {
        ContractType::Raw => {
            let ffi = impl_trait.gen_extern_functions();
            return quote! {
                #input
                #ffi
            }
            .into();
        }
        ContractType::Composable => {}
    }

    #[derive(Default)]
    struct AsocTypesParse {
        params: Option<Type>,
        delta: Option<Type>,
        summary: Option<Type>,
    }
    let mut asoc_types_opt = AsocTypesParse::default();
    for item in &input.items {
        if let ImplItem::Type(asoc_type) = item {
            match asoc_type.ident.to_string().as_str() {
                "Parameters" => {
                    asoc_types_opt.params = Some(asoc_type.ty.clone());
                }
                "Delta" => {
                    asoc_types_opt.delta = Some(asoc_type.ty.clone());
                }
                "Summary" => {
                    asoc_types_opt.summary = Some(asoc_type.ty.clone());
                }
                _ => {}
            }
        }
    }

    let asoc_types = AsocTypes {
        params: {
            let Some(p) = asoc_types_opt.params else {
                return quote_spanned! {
                    attr_span =>
                    compile_error!("missing Parameters associated type");
                }
                .into();
            };
            p
        },
        delta: {
            let Some(p) = asoc_types_opt.delta else {
                return quote_spanned! {
                    attr_span =>
                    compile_error!("missing Delta associated type");
                }
                .into();
            };
            p
        },
        summary: {
            let Some(p) = asoc_types_opt.summary else {
                return quote_spanned! {
                    attr_span =>
                    compile_error!("missing Summary associated type");
                }
                .into();
            };
            p
        },
    };

    let mut children: Vec<TypePath> = vec![];
    let mut found_children = false;
    let mut encoder = None;

    for m in args.args.iter() {
        match m {
            syn::Meta::List(list) => match list.path.get_ident() {
                Some(id) if id == "children" => {
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
                }
                Some(other) => {
                    return quote_spanned! {
                        other.span() =>
                        compile_error!("this argument list is not allowed, must be: `children`");
                    }
                    .into();
                }
                None => {
                    return quote_spanned! {
                        list.span() =>
                        compile_error!("expected a list identifier");
                    }
                    .into();
                }
            },
            syn::Meta::NameValue(MetaNameValue {
                path,
                value: syn::Expr::Path(type_path),
                ..
            }) if path.get_ident().map(|id| id == "encoder").unwrap_or(false) => {
                if encoder.is_some() {
                    return quote_spanned! {
                        path.span() =>
                        compile_error!("only one encoder protocol can be specified");
                    }
                    .into();
                }
                encoder = Some(&type_path.path);
            }
            other => {
                return quote_spanned! {
                    other.span() =>
                    compile_error!("argument not allowed");
                }
                .into()
            }
        }
    }

    impl_trait.children = children;

    if encoder.is_none() {
        return quote_spanned! {
            attr_span =>
            compile_error!("at least one encoder must be specified, possible protocols: BincodeEncoder");
        }
        .into();
    }
    let encoder = encoder.unwrap();

    if !found_children {
        todo!("impl encoder only")
    } else if impl_trait.children.is_empty() {
        return quote_spanned! {
            attr_span =>
            compile_error!("at least one ComposableContract child is required");
        }
        .into();
    } else {
        let contract_iface = impl_trait.gen_contract_iface(encoder);
        let ffi = impl_trait.gen_extern_functions();
        let encoder_impl = impl_trait.encoder_impl(encoder, &asoc_types);
        let serialization_adapter = impl_trait.gen_serialization_adapter(&asoc_types);
        quote! {
            #input
            #contract_iface
            #ffi
            #encoder_impl
            #serialization_adapter
        }
        .into()
    }
}

struct ImplTrait {
    type_name: TypePath,
    children: Vec<TypePath>,
}

impl ImplTrait {
    fn ffi_ret_type(&self) -> TokenStream {
        quote!(i64)
    }

    fn gen_serialization_adapter(&self, asoc_types: &AsocTypes) -> TokenStream {
        let type_name = &self.type_name;
        let params = &asoc_types.params;
        let delta = &asoc_types.delta;
        let summary = &asoc_types.summary;
        quote! {
            impl ::freenet_stdlib::prelude::SerializationAdapter for #type_name {
                type Parameters = #params;
                type Delta = #delta;
                type Summary = #summary;
            }
        }
    }

    fn encoder_impl(&self, encoder: &syn::Path, asoc_types: &AsocTypes) -> TokenStream {
        let type_name = &self.type_name;
        let params = &asoc_types.params;
        let delta = &asoc_types.delta;
        let summary = &asoc_types.summary;
        quote! {
            impl #encoder for #type_name {}
            impl #encoder for #params {}
            impl #encoder for #delta {}
            impl #encoder for #summary {}
        }
    }

    fn gen_contract_iface(&self, encoder: &syn::Path) -> TokenStream {
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
                    let encoded_summary = #encoder::serialize(&serializable_summary)?;
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
                    let encoded_delta = #encoder::serialize(&serializable_delta)?;
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
