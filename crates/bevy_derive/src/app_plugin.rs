use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[deprecated(
    since = "0.14.0",
    note = "The current dynamic plugin system is unsound and will be removed in 0.15."
)]
pub fn derive_dynamic_plugin(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        #[no_mangle]
        pub extern "C" fn _bevy_create_plugin() -> *mut dyn bevy::app::Plugin {
            // make sure the constructor is the correct type.
            let object = #struct_name {};
            let boxed = Box::new(object);
            Box::into_raw(boxed)
        }
    })
}
