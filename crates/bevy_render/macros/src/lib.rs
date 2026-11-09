#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod as_bind_group;
mod specializer;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_render"))
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_ecs"))
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
