use quote::{quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ItemImpl, Meta, Token};

pub(crate) mod common;
mod contract_impl;
mod delegate_impl;

struct AttributeArgs {
    args: Punctuated<Meta, Token![,]>,
}

impl syn::parse::Parse for AttributeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Punctuated::new();
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
            let meta = input.parse::<Meta>()?;
            args.push(meta);
            count += 1;
        }
        Ok(AttributeArgs { args })
    }
}

enum ContractType {
    Raw,
    Typed,
    Composable,
}

/// Generate the necessary code for the WASM runtime to interact with your contract ergonomically and safely.
#[proc_macro_attribute]
pub fn contract(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(args as AttributeArgs);
    let input = syn::parse_macro_input!(input as ItemImpl);
    let Some((_, path, _)) = &input.trait_ else {
        return proc_macro::TokenStream::from(quote_spanned! {
            input.span() =>
            compile_error!("only allowed for traits");
        });
    };
    match path.segments.last() {
        Some(segment) => {
            let c_type = match segment.ident.to_string().as_str() {
                "ContractInterface" => ContractType::Raw,
                "TypedContract" => ContractType::Typed,
                "ContractComponent" => ContractType::Composable,
                _ => {
                    return proc_macro::TokenStream::from(quote_spanned! {
                        segment.ident.span() =>
                        compile_error!("trait not supported for contract interaction");
                    })
                }
            };
            contract_impl::contract_ffi_impl(&input, &args, c_type)
        }
        None => proc_macro::TokenStream::from(quote_spanned! {
            path.span() =>
            compile_error!("missing trait identifier");
        }),
    }
}

/// Generate the necessary code for the WASM runtime to interact with your contract ergonomically and safely.
#[proc_macro_attribute]
pub fn delegate(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _args = syn::parse_macro_input!(args as AttributeArgs);
    let input = syn::parse_macro_input!(input as ItemImpl);
    let output = delegate_impl::ffi_impl_wrap(&input);
    // println!("{}", quote!(#input));
    // println!("{output}");
    proc_macro::TokenStream::from(quote! {
        #input
        #output
    })
}
