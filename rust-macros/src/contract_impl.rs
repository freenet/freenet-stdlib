use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, Type, TypePath};

pub fn ffi_impl_wrap(item: &ItemImpl) -> TokenStream {
    let type_name = match &*item.self_ty {
        Type::Path(p) => p.clone(),
        _ => panic!(),
    };
    let s = ImplStruct { type_name };
    let validate_state_fn = s.gen_validate_state_fn();
    let validate_delta_fn = s.gen_validate_delta_fn();
    let update_fn = s.gen_update_state_fn();
    let summarize_fn = s.gen_summarize_state_fn();
    let get_delta_fn = s.gen_get_state_delta();
    let result = quote! {
        #validate_state_fn
        #validate_delta_fn
        #update_fn
        #summarize_fn
        #get_delta_fn
    };
    // println!("{result}");
    result
}

struct ImplStruct {
    type_name: TypePath,
}

impl ImplStruct {
    fn ffi_ret_type(&self) -> TokenStream {
        quote!(i64)
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
