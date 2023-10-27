use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, Data, DataEnum, DataStruct, Error, LitInt, Result};

pub fn derive_pipeline_key(ast: syn::DeriveInput, render_path: syn::Path) -> Result<TokenStream> {
    let manifest = BevyManifest::default();
    let utils_path = manifest.get_path("bevy_utils");

    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    let bits = match &ast.data {
        Data::Enum(DataEnum { variants, .. }) => {
            for variant in variants {
                if !variant.fields.is_empty() {
                    return Err(Error::new_spanned(ast, "Expected a unit enum"));
                }
            }

            let count = variants.len();
            let bits = ((count - 1).ilog2() + 1) as u8;

            quote!(#bits)
        }
        Data::Struct(DataStruct {
            fields,
            ..
        }) => {
            let bits = fields.iter().map(|field| {
                let ty = &field.ty;
                quote!(<#ty as #render_path::pipeline_keys::FixedSizePipelineKey>::size())
            });

            quote!((#(#bits + )* 0))
        }
        _ => {
            let Some(bits) = ast
                .attrs
                .iter()
                .find(|attr| attr.path().get_ident().map_or(false, |ident| ident == "key_bits"))
                .and_then(|attr| {
                    attr.parse_args_with(LitInt::parse)
                        .ok()
                        .and_then(|lit| lit.base10_parse::<u8>().ok())
                })
            else {
                return Err(Error::new_spanned(
                    ast,
                    "PipelineKey unions must have a #[key_bits(n)] attribute to declare their size",
                ));
            };

            quote!(#bits)
        }
    };

    Ok(TokenStream::from(quote! {
        impl #impl_generics #render_path::pipeline_keys::KeyType for #struct_name #ty_generics #where_clause {
            fn as_any(&self) -> &dyn core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
                self
            }

            fn positions(&self) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                #utils_path::HashMap::from_iter([(core::any::TypeId::of::<Self>(), #render_path::pipeline_keys::SizeOffset(#bits, 0u8))])
            }
        }
    }))
}
