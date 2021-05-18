use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

pub fn derive_from_resources(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    let bevy_app_path = BevyManifest::default().get_path(crate::modules::BEVY_APP);
    let field_types = fields.iter().map(|field| &field.ty);
    let fields = fields.iter().map(|field| field.ident.as_ref().unwrap());
    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_app_path::FromResources for #struct_name#ty_generics {
            fn from_resources(resources: &Resources) -> Self {
                use #bevy_app_path::FromResources;
                #struct_name {
                    #(#fields: <#field_types>::from_resources(resources),)*
                }
            }
        }
    })
}
