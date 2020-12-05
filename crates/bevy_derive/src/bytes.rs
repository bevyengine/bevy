use crate::modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

pub fn derive_bytes(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    let modules = get_modules(&ast.attrs);
    let bevy_core_path = get_path(&modules.bevy_core);

    let fields = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_core_path::Bytes for #struct_name#ty_generics {
            fn write_bytes(&self, buffer: &mut [u8]) {
                let mut offset: usize = 0;
                #(let byte_len = self.#fields.byte_len();
                self.#fields.write_bytes(&mut buffer[offset..(offset + byte_len)]);
                offset += byte_len;)*
            }
            fn byte_len(&self) -> usize {
                let mut byte_len: usize = 0;
                #(byte_len += self.#fields.byte_len();)*
                byte_len
            }
        }
    })
}
