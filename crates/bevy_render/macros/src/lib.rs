#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod as_bind_group;
mod extract_component;
mod extract_resource;
mod specializer;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::shared().get_path("bevy_render")
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::shared().get_path("bevy_ecs")
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
