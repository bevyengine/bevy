use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{Data, DataEnum, DataStruct, Error, Result, Fields};

pub fn derive_pipeline_key(ast: syn::DeriveInput, render_path: syn::Path) -> Result<TokenStream> {
    let manifest = BevyManifest::default();
    let utils_path = manifest.get_path("bevy_utils");

    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    match &ast.data {
        Data::Enum(DataEnum { variants, .. }) => {
            for variant in variants {
                if !variant.fields.is_empty() {
                    return Err(Error::new_spanned(ast, "PipelineKey target must be either a unit enum or a struct of KeyTypes"));
                }
            }

            let count = variants.len();
            let bits = ((count - 1).ilog2() + 1) as u8;

            Ok(TokenStream::from(quote! {
                impl #impl_generics #render_path::pipeline_keys::AnyKeyType for #struct_name #ty_generics #where_clause {
                    fn as_any(&self) -> &dyn core::any::Any {
                        self
                    }
                }
        
                impl #impl_generics #render_path::pipeline_keys::KeyTypeConcrete for #struct_name #ty_generics #where_clause {
                    fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                        #utils_path::HashMap::from_iter([(core::any::TypeId::of::<Self>(), #render_path::pipeline_keys::SizeOffset(#bits, 0u8))])
                    }
        
                    fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                        #render_path::pipeline_keys::PackedPipelineKey::new(u32::from(*value), #bits)
                    }
        
                    fn unpack(value: u32, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                        value.into()
                    }
                }

                impl #impl_generics #render_path::pipeline_keys::FixedSizeKey for #struct_name #ty_generics #where_clause {
                    fn fixed_size() -> u8 {
                        #bits
                    }
                }
            }))
        }
        Data::Struct(DataStruct {
            fields,
            ..
        }) => {
            let is_dynamic = ast.attrs.iter().any(|attr| attr.meta.path().get_ident() == Some(&format_ident!("dynamic_key")));
            let is_not_fixed_size = ast.attrs.iter().any(|attr| attr.meta.path().get_ident() == Some(&format_ident!("not_fixed_size")));

            if is_dynamic {
                if fields.len() != 1 {
                    return Err(Error::new_spanned(
                        ast,
                        "dynamic PipelineKeys must be newtype structs around the pipeline key primitive",
                    ))
                }

                let Fields::Unnamed(_) = fields else {
                    return Err(Error::new_spanned(
                        ast,
                        "dynamic PipelineKeys must be newtype structs around the pipeline key primitive",
                    ))
                };

                return Ok(TokenStream::from(quote! {
                    impl #impl_generics #render_path::pipeline_keys::AnyKeyType for #struct_name #ty_generics #where_clause {
                        fn as_any(&self) -> &dyn core::any::Any {
                            self
                        }
                    }
            
                    impl #impl_generics #render_path::pipeline_keys::KeyTypeConcrete for #struct_name #ty_generics #where_clause {
                        fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                            let res = store.meta::<Self>().dynamic_components.clone();
                            res
                        }
    
                        fn size(store: &#render_path::pipeline_keys::KeyMetaStore) -> u8 {
                            store.meta::<Self>().size
                        }
            
                        fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                            #render_path::pipeline_keys::PackedPipelineKey::new(value.0, Self::size(store))
                        }
            
                        fn unpack(value: u32, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                            Self(value)
                        }
                    }

                    impl #impl_generics #render_path::pipeline_keys::DynamicKey for #struct_name #ty_generics #where_clause {}
                }));
            }

            let field_exprs = fields.iter().enumerate().map(|(i, f)| {
                f.ident.clone().map(|ident| quote! { value.#ident} ).unwrap_or_else(|| {
                    let i = syn::Index::from(i);
                    quote! { value.#i }
                })
            }).collect::<Vec<_>>();

            let field_names = fields.iter().enumerate().map(|(i, f)| {
                let ident = f.ident.clone().unwrap_or_else(|| format_ident!("value_{i}"));
                quote!{ #ident }
            }).collect::<Vec<_>>();

            let self_value = if fields.iter().any(|f| f.ident.is_none()) {
                quote!{ Self(#(#field_names),*)}
            } else {
                quote!{ Self { #(#field_names),* }}
            };

            let field_types = fields.iter().map(|f| f.ty.clone()).collect::<Vec<_>>();

            let fixed_size_impl = if is_not_fixed_size {
                quote!()
            } else {
                quote!{
                    impl #impl_generics #render_path::pipeline_keys::FixedSizeKey for #struct_name #ty_generics #where_clause {
                        fn fixed_size() -> u8 {
                            #(#field_types::fixed_size())+*
                        }
                    }
                }
            };

            Ok(TokenStream::from(quote! {
                impl #impl_generics #render_path::pipeline_keys::AnyKeyType for #struct_name #ty_generics #where_clause {
                    fn as_any(&self) -> &dyn core::any::Any {
                        self
                    }
                }
        
                impl #impl_generics #render_path::pipeline_keys::KeyTypeConcrete for #struct_name #ty_generics #where_clause {
                    fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                        #utils_path::HashMap::from_iter([(core::any::TypeId::of::<Self>(), #render_path::pipeline_keys::SizeOffset(Self::size(store), 0u8))])
                    }

                    fn size(store: &#render_path::pipeline_keys::KeyMetaStore) -> u8 {
                        #(#field_types::size(store))+*
                    }
        
                    fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                        let tuple = (#(#field_exprs,)*);
                        let #render_path::pipeline_keys::PackedPipelineKey{ packed, size, .. } = #render_path::pipeline_keys::KeyTypeConcrete::pack(&tuple, store);
                        #render_path::pipeline_keys::PackedPipelineKey::new(packed, size)
                    }
        
                    fn unpack(value: u32, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                        let (#(#field_names,)*) = #render_path::pipeline_keys::KeyTypeConcrete::unpack(value, store);
                        #self_value
                    }
                }

                #fixed_size_impl
            }))
        }
        _ => Err(Error::new_spanned(
            ast,
            "PipelineKey target must be either a unit enum or a struct of KeyTypes",
        ))
    }
}
