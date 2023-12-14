use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_quote, Data, DataEnum, DataStruct, Error, Fields, PathArguments, PathSegment, Result,
};

pub fn derive_pipeline_key(ast: syn::DeriveInput, render_path: syn::Path) -> Result<TokenStream> {
    let manifest = BevyManifest::default();
    let utils_path = manifest.get_path("bevy_utils");
    let ecs_path = manifest.get_path("bevy_ecs");

    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    let custom_defs = ast
        .attrs
        .iter()
        .any(|attr| attr.meta.path().get_ident() == Some(&format_ident!("custom_shader_defs")));

    match &ast.data {
        Data::Enum(DataEnum { variants, .. }) => {
            let mut variant_numbers = Vec::default();
            let mut variant_names = Vec::default();
            for (i, variant) in variants.iter().enumerate() {
                if !variant.fields.is_empty() {
                    return Err(Error::new_spanned(
                        variant,
                        "PipelineKey target must be either a unit enum or a struct of KeyTypes",
                    ));
                }
                if variant.discriminant.is_some() {
                    return Err(Error::new_spanned(
                        variant,
                        "no explicit discriminants please, we're pipeline keys",
                    ));
                }
                let name = &variant.ident;
                variant_numbers.push(quote! { #i });
                variant_names.push(quote! { #name });
            }

            let Some(repr_attr) = ast
                .attrs
                .iter()
                .find(|attr| attr.meta.path().get_ident() == Some(&format_ident!("repr")))
            else {
                return Err(Error::new_spanned(
                    ast,
                    "PipelineKey enum requires a #[repr({integer})] annotation",
                ));
            };
            let syn::Meta::List(meta_list) = &repr_attr.meta else {
                return Err(Error::new_spanned(
                    repr_attr,
                    "repr needs exactly one argument",
                ));
            };
            let mut meta_list = meta_list.tokens.clone().into_iter();
            let (Some(repr), None) = (meta_list.next(), meta_list.next()) else {
                return Err(Error::new_spanned(
                    repr_attr,
                    "repr needs exactly one argument",
                ));
            };
            let repr: syn::Ident = parse_quote! { #repr };

            let count = variants.len();
            let bits = ((count - 1).ilog2() + 1) as u8;

            let defs_impl = if !custom_defs {
                quote! {
                    fn shader_defs(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Vec<#render_path::render_resource::ShaderDefVal> {
                        Vec::default()
                    }
                }
            } else {
                quote! {
                    fn shader_defs(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Vec<#render_path::render_resource::ShaderDefVal> {
                        <Self as #render_path::pipeline_keys::KeyShaderDefs>::shader_defs(&Self::unpack(value, store))
                    }
                }
            };

            let custom_defs_impl = if custom_defs {
                quote! {}
            } else {
                // we implement the KeyShaderDefs trait here to force an error if it is manually implemented without #[custom_shader_defs]
                quote! {
                    impl #impl_generics #render_path::pipeline_keys::KeyShaderDefs for #struct_name #ty_generics #where_clause {
                        fn shader_defs(&self) -> Vec<#render_path::render_resource::ShaderDefVal> {
                            Vec::default()
                        }
                    }
                }
            };

            Ok(TokenStream::from(quote! {
                impl #impl_generics #render_path::pipeline_keys::PipelineKeyType for #struct_name #ty_generics #where_clause {
                    fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                        #utils_path::HashMap::from_iter([(core::any::TypeId::of::<Self>(), #render_path::pipeline_keys::SizeOffset{ size: #bits, offset: 0u8 })])
                    }

                    fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                        #render_path::pipeline_keys::PackedPipelineKey::new((*value as #repr) as #render_path::pipeline_keys::KeyPrimitive, #bits)
                    }

                    fn unpack(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                        Self::from(value as #repr)
                    }

                    #defs_impl
                }

                impl #impl_generics #render_path::pipeline_keys::FixedSizeKey for #struct_name #ty_generics #where_clause {
                    fn fixed_size() -> u8 {
                        #bits
                    }
                }

                impl #impl_generics ::core::convert::From<#repr> for #struct_name #ty_generics #where_clause {
                    #[inline]
                    fn from (
                        number: #repr,
                    ) -> Self {
                        match number as usize {
                            #(
                                #variant_numbers => Self::#variant_names,
                            )*
                            #[allow(unreachable_patterns)]
                            _ => panic!("unexpected value in from"),
                        }
                    }
                }

                #custom_defs_impl
            }))
        }
        Data::Struct(DataStruct { fields, .. }) => {
            let is_dynamic = ast
                .attrs
                .iter()
                .any(|attr| attr.meta.path().get_ident() == Some(&format_ident!("dynamic_key")));
            let is_not_fixed_size = ast
                .attrs
                .iter()
                .any(|attr| attr.meta.path().get_ident() == Some(&format_ident!("not_fixed_size")));

            let defs_impl = if !custom_defs {
                // use the default impl
                quote! {}
            } else {
                quote! {
                    fn shader_defs(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Vec<#render_path::render_resource::ShaderDefVal> {
                        <Self as #render_path::pipeline_keys::KeyShaderDefs>::shader_defs(&Self::unpack(value, store))
                    }
                }
            };

            let custom_defs_impl = if custom_defs {
                quote!()
            } else {
                quote! {
                    impl #impl_generics #render_path::pipeline_keys::KeyShaderDefs for #struct_name #ty_generics #where_clause {
                        fn shader_defs(&self) -> Vec<#render_path::render_resource::ShaderDefVal> {
                            Vec::default()
                        }
                    }
                }
            };

            if is_dynamic {
                if fields.len() != 1 {
                    return Err(Error::new_spanned(
                        ast,
                        "dynamic PipelineKeys must be newtype structs around the pipeline key primitive",
                    ));
                }

                let Fields::Unnamed(_) = fields else {
                    return Err(Error::new_spanned(
                        ast,
                        "dynamic PipelineKeys must be newtype structs around the pipeline key primitive",
                    ));
                };

                return Ok(TokenStream::from(quote! {
                    impl #impl_generics #render_path::pipeline_keys::PipelineKeyType for #struct_name #ty_generics #where_clause {
                        fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                            store.meta::<Self>().dynamic_components.clone()
                        }

                        fn size(store: &#render_path::pipeline_keys::KeyMetaStore) -> u8 {
                            store.meta::<Self>().size
                        }

                        fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                            #render_path::pipeline_keys::PackedPipelineKey::new(value.0, <Self as #render_path::pipeline_keys::PipelineKeyType>::size(store))
                        }

                        fn unpack(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                            Self(value)
                        }

                        #defs_impl
                    }

                    impl #impl_generics #render_path::pipeline_keys::DynamicKey for #struct_name #ty_generics #where_clause {}

                    #custom_defs_impl
                }));
            }

            let field_exprs = fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    f.ident
                        .clone()
                        .map(|ident| quote! { value.#ident})
                        .unwrap_or_else(|| {
                            let i = syn::Index::from(i);
                            quote! { value.#i }
                        })
                })
                .collect::<Vec<_>>();

            let field_names = fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let ident = f
                        .ident
                        .clone()
                        .unwrap_or_else(|| format_ident!("value_{i}"));
                    quote! { #ident }
                })
                .collect::<Vec<_>>();

            let self_value = if fields.iter().any(|f| f.ident.is_none()) {
                quote! { Self(#(#field_names,)*)}
            } else {
                quote! { Self { #(#field_names,)* }}
            };

            let field_types = fields
                .iter()
                .map(|f| {
                    // turn Option<T> into Option::<T> so we can call functions on it
                    fn colonize_type(ty: &mut syn::Type) {
                        if let syn::Type::Path(ref mut typath) = ty {
                            for segment in &mut typath.path.segments {
                                colonize_segment(segment);
                            }
                        }
                    }

                    fn colonize_segment(segment: &mut PathSegment) {
                        let span = segment.ident.span();
                        if let PathArguments::AngleBracketed(ref mut abgis) = &mut segment.arguments
                        {
                            abgis.colon2_token = Some(syn::token::PathSep {
                                spans: [span, span],
                            });
                            for mut arg in &mut abgis.args {
                                if let syn::GenericArgument::Type(ref mut ty) = &mut arg {
                                    colonize_type(ty);
                                }
                            }
                        }
                    }

                    let mut ty = f.ty.clone();
                    colonize_type(&mut ty);
                    ty
                })
                .collect::<Vec<_>>();

            let fixed_size_impl = if is_not_fixed_size {
                quote!()
            } else {
                quote! {
                    impl #impl_generics #render_path::pipeline_keys::FixedSizeKey for #struct_name #ty_generics #where_clause {
                        fn fixed_size() -> u8 {
                            #(#field_types::fixed_size() )+*
                        }
                    }
                }
            };

            Ok(TokenStream::from(quote! {
                impl #impl_generics #render_path::pipeline_keys::PipelineKeyType for #struct_name #ty_generics #where_clause {
                    fn positions(store: &#render_path::pipeline_keys::KeyMetaStore) -> #utils_path::HashMap<core::any::TypeId, #render_path::pipeline_keys::SizeOffset> {
                        <( #(#field_types,)* ) as #render_path::pipeline_keys::PipelineKeyType>::positions(store)
                    }

                    fn size(store: &#render_path::pipeline_keys::KeyMetaStore) -> u8 {
                        #(<#field_types as #render_path::pipeline_keys::PipelineKeyType>::size(store))+*
                    }

                    fn pack(value: &Self, store: &#render_path::pipeline_keys::KeyMetaStore) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                        let mut result = 0 as #render_path::pipeline_keys::KeyPrimitive;
                        let mut total_size = 0u8;

                        #(
                            let #render_path::pipeline_keys::PackedPipelineKey{ packed, size, .. } = #render_path::pipeline_keys::PipelineKeyType::pack(&#field_exprs, store);
                            result = (result << size) | packed;
                            total_size += size;
                        )*

                        #render_path::pipeline_keys::PackedPipelineKey::new(result, total_size)
                    }

                    fn unpack(value: #render_path::pipeline_keys::KeyPrimitive, store: &#render_path::pipeline_keys::KeyMetaStore) -> Self {
                        let (#(#field_names,)*) = #render_path::pipeline_keys::PipelineKeyType::unpack(value, store);
                        #self_value
                    }

                    #defs_impl
                }

                #fixed_size_impl

                #custom_defs_impl

                impl #impl_generics #render_path::pipeline_keys::KeyRepack for #struct_name #ty_generics #where_clause {
                    type PackedParts = (#(#render_path::pipeline_keys::PackedPipelineKey<#field_types>,)*);

                    fn repack(source: Self::PackedParts) -> #render_path::pipeline_keys::PackedPipelineKey<Self> {
                        let #render_path::pipeline_keys::PackedPipelineKey{ packed, size, .. } = <( #(#field_types,)* ) as #render_path::pipeline_keys::KeyRepack>::repack(source);
                        #render_path::pipeline_keys::PackedPipelineKey::new(packed, size)
                    }
                }

                impl #impl_generics #render_path::pipeline_keys::CompositeKey for #struct_name #ty_generics #where_clause {
                    fn from_keys(keys: &#render_path::pipeline_keys::PipelineKeys) -> Option<#render_path::pipeline_keys::PackedPipelineKey<Self>> {
                        if let Some(#render_path::pipeline_keys::PackedPipelineKey { packed, size, .. }) = <(#(#field_types,)*) as #render_path::pipeline_keys::CompositeKey>::from_keys(keys) {
                            Some(#render_path::pipeline_keys::PackedPipelineKey::new(packed, size))
                        } else {
                            None
                        }
                    }

                    fn set_config() -> #ecs_path::schedule::NodeConfigs<#utils_path::intern::Interned<dyn #ecs_path::schedule::SystemSet>> {
                        <(#(#field_types,)*) as #render_path::pipeline_keys::CompositeKey>::set_config()
                    }
                }
            }))
        }
        _ => Err(Error::new_spanned(
            ast,
            "PipelineKey target must be either a unit enum or a struct of KeyTypes",
        )),
    }
}
