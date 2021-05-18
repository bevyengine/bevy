use bevy_macro_utils::BevyManifest;
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

    let bevy_core_path = BevyManifest::default().get_path(crate::modules::BEVY_CORE);

    let fields = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let struct_name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_core_path::Bytes for #struct_name #ty_generics #where_clause {
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
