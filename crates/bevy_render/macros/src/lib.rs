#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod as_bind_group;
mod extract_component;
mod extract_resource;
mod specializer;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_render"))
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_ecs"))
}

#[proc_macro_derive(ExtractResource)]
pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    extract_resource::derive_extract_resource(input)
}

/// Implements `ExtractComponent` trait for a component.
///
/// The component must implement [`Clone`].
/// The component will be extracted into the render world via cloning.
/// Note that this only enables extraction of the component, it does not execute the extraction.
/// See `ExtractComponentPlugin` to actually perform the extraction.
///
/// If you only want to extract a component conditionally, you may use the `extract_component_filter` attribute.
///
/// # Example
///
/// ```no_compile
/// use bevy_ecs::component::Component;
/// use bevy_render_macros::ExtractComponent;
///
/// #[derive(Component, Clone, ExtractComponent)]
/// #[extract_component_filter(With<Camera>)]
/// pub struct Foo {
///     pub should_foo: bool,
/// }
///
/// // Without a filter (unconditional).
/// #[derive(Component, Clone, ExtractComponent)]
/// pub struct Bar {
///     pub should_bar: bool,
/// }
/// ```
#[proc_macro_derive(ExtractComponent, attributes(extract_component_filter))]
pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    extract_component::derive_extract_component(input)
}

#[proc_macro_derive(
    AsBindGroup,
    attributes(
        uniform,
        storage_texture,
        texture,
        sampler,
        bind_group_data,
        storage,
        bindless,
        data
    )
)]
pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    as_bind_group::derive_as_bind_group(input).unwrap_or_else(|err| err.to_compile_error().into())
}

/// Derive macro generating an impl of the trait `RenderLabel`.
///
/// This does not work for unions.
#[proc_macro_derive(RenderLabel)]
pub fn derive_render_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_graph").into());
    trait_path
        .segments
        .push(format_ident!("RenderLabel").into());
    derive_label(input, "RenderLabel", &trait_path)
}

/// Derive macro generating an impl of the trait `RenderSubGraph`.
///
/// This does not work for unions.
#[proc_macro_derive(RenderSubGraph)]
pub fn derive_render_sub_graph(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_graph").into());
    trait_path
        .segments
        .push(format_ident!("RenderSubGraph").into());
    derive_label(input, "RenderSubGraph", &trait_path)
}

/// Derive macro generating an impl of the trait `Specializer`
///
/// This only works for structs whose members all implement `Specializer`
#[proc_macro_derive(Specializer, attributes(specialize, key, base_descriptor))]
pub fn derive_specialize(input: TokenStream) -> TokenStream {
    specializer::impl_specializer(input)
}

/// Derive macro generating the most common impl of the trait `SpecializerKey`
#[proc_macro_derive(SpecializerKey)]
pub fn derive_specializer_key(input: TokenStream) -> TokenStream {
    specializer::impl_specializer_key(input)
}

#[proc_macro_derive(ShaderLabel)]
pub fn derive_shader_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_phase").into());
    trait_path
        .segments
        .push(format_ident!("ShaderLabel").into());
    derive_label(input, "ShaderLabel", &trait_path)
}

#[proc_macro_derive(DrawFunctionLabel)]
pub fn derive_draw_function_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_phase").into());
    trait_path
        .segments
        .push(format_ident!("DrawFunctionLabel").into());
    derive_label(input, "DrawFunctionLabel", &trait_path)
}

/// Implement `BinnedPhaseItem` and other related traits for a newtype.
#[proc_macro_derive(BinnedPhaseItem)]
pub fn derive_binned_phase_item(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let inner_ty = match extract_newtype_inner(&ast) {
        Ok(ty) => ty,
        Err(err) => return err.to_compile_error().into(),
    };

    let bevy_render = bevy_render_path();
    let bevy_ecs = bevy_ecs_path();

    let struct_name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let phase_item_impl = impl_phase_item(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        &bevy_render,
        &bevy_ecs,
    );

    let binned_phase_item_impl = impl_binned_phase_item(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        inner_ty,
        &bevy_render,
        &bevy_ecs,
    );

    let cached_pipeline_impl = impl_cached_pipeline(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        &bevy_render,
    );

    TokenStream::from(quote! {
        #phase_item_impl
        #binned_phase_item_impl
        #cached_pipeline_impl
    })
}

/// Implement `SortedPhaseItem` and other related traits for a newtype.
///
/// NOTE: Currently, we are using the default implementation of `sort` for `SortedPhaseItem`.
#[proc_macro_derive(SortedPhaseItem)]
pub fn derive_sorted_phase_item(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let inner_ty = match extract_newtype_inner(&ast) {
        Ok(ty) => ty,
        Err(err) => return err.to_compile_error().into(),
    };

    let bevy_render = bevy_render_path();
    let bevy_ecs = bevy_ecs_path();

    let struct_name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let phase_item_impl = impl_phase_item(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        &bevy_render,
        &bevy_ecs,
    );

    let sorted_phase_item_impl = impl_sorted_phase_item(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        inner_ty,
        &bevy_render,
    );

    let cached_pipeline_impl = impl_cached_pipeline(
        struct_name,
        &impl_generics,
        &type_generics,
        where_clause,
        &bevy_render,
    );

    TokenStream::from(quote! {
        #phase_item_impl
        #sorted_phase_item_impl
        #cached_pipeline_impl
    })
}

fn extract_newtype_inner(ast: &DeriveInput) -> syn::Result<&syn::Type> {
    match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                Ok(&fields.unnamed.first().unwrap().ty)
            }
            _ => Err(syn::Error::new_spanned(
                ast.ident.clone(),
                "#[derive(Binned)] requires a single-field tuple struct",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            ast.ident.clone(),
            "#[derive(Binned)] can only be used on structs",
        )),
    }
}

fn impl_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    bevy_render: &syn::Path,
    bevy_ecs: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::PhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn entity(&self) -> #bevy_ecs::entity::Entity {
                self.0.entity()
            }

            #[inline]
            fn main_entity(&self) -> #bevy_render::sync_world::MainEntity {
                self.0.main_entity()
            }

            #[inline]
            fn draw_function(&self) -> #bevy_render::render_phase::DrawFunctionId {
                self.0.draw_function()
            }

            #[inline]
            fn batch_range(&self) -> &::core::ops::Range<u32> {
                self.0.batch_range()
            }

            #[inline]
            fn batch_range_mut(&mut self) -> &mut ::core::ops::Range<u32> {
                self.0.batch_range_mut()
            }

            #[inline]
            fn extra_index(&self) -> #bevy_render::render_phase::PhaseItemExtraIndex {
                self.0.extra_index()
            }

            #[inline]
            fn batch_range_and_extra_index_mut(
                &mut self,
            ) -> (
                &mut ::core::ops::Range<u32>,
                &mut #bevy_render::render_phase::PhaseItemExtraIndex
            ) {
                self.0.batch_range_and_extra_index_mut()
            }
        }
    }
}

fn impl_binned_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_render: &syn::Path,
    bevy_ecs: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::BinnedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            type BatchSetKey =
                <#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::BatchSetKey;
            type BinKey =
                <#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::BinKey;

            #[inline]
            fn new(
                batch_set_key: Self::BatchSetKey,
                bin_key: Self::BinKey,
                representative_entity: (#bevy_ecs::entity::Entity, #bevy_render::sync_world::MainEntity),
                batch_range: ::core::ops::Range<u32>,
                extra_index: #bevy_render::render_phase::PhaseItemExtraIndex,
            ) -> Self {
                Self(<#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::new(
                    batch_set_key,
                    bin_key,
                    representative_entity,
                    batch_range,
                    extra_index,
                ))
            }
        }
    }
}

fn impl_sorted_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_render: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::SortedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            type SortKey =
                <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::SortKey;

            #[inline]
            fn sort_key(&self) -> Self::SortKey {
                <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::sort_key(&self.0)
            }

            // NOTE: Currently, we are using the default implementation of `sort`. To address this,
            // a new associated type `SortItem` needs to be added for `SortedPhaseItem`.
            // #[inline]
            // fn sort(items: &mut [Self]) {
            //     <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::sort(items)
            // }

            #[inline]
            fn indexed(&self) -> bool {
                self.0.indexed()
            }
        }
    }
}

fn impl_cached_pipeline(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    bevy_render: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::CachedRenderPipelinePhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn cached_pipeline(&self) -> #bevy_render::render_resource::CachedRenderPipelineId {
                self.0.cached_pipeline()
            }
        }
    }
}
