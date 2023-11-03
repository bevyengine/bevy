mod as_bind_group;
mod extract_component;
mod extract_resource;
mod pipeline_key;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_macro_input, token::Crate, DeriveInput};

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::default()
        .maybe_get_path("bevy_render")
        // NOTE: If the derivation is within bevy_render, then we need to return 'crate'
        .unwrap_or_else(|| BevyManifest::parse_str("crate"))
}

#[proc_macro_derive(ExtractResource)]
pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    extract_resource::derive_extract_resource(input)
}

/// Implements `ExtractComponent` trait for a component.
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
    attributes(uniform, texture, sampler, bind_group_data, storage)
)]
pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    as_bind_group::derive_as_bind_group(input).unwrap_or_else(|err| err.to_compile_error().into())
}

/// Implements `PipelineKeyType` trait for an enum or struct.
/// Enums must be unit enums, and will be packed into the number of bits required by the variant count. They must
/// also be annotated with a `#[repr(u8)]` (or larger integer type if required) attribute.
///
/// Structs must contain packable types (other `PipelineKey`s, bool, u8, u32 primitives, and some wgpu types, see
/// `bevy_render::pipeline_keys::packed_types`)
///
/// Dynamic keys can be derived with the `#[dynamic_key]` attribute.
/// Keys that provide `ShaderDefVal`s must use the `#[custom_shader_defs]` attribute, and manually implement the `KeyShaderDefs` trait.
/// Keys that contain dynamic keys must use the `#[not_fixed_size]` attribute.
///
/// You can also implement `SystemKey` trait for structs or enums to enable automatic calculation of the key value for matching entities,
/// see the `AddPipelineKey` trait for further details.
#[proc_macro_derive(
    PipelineKey,
    attributes(dynamic_key, not_fixed_size, custom_shader_defs)
)]
pub fn derive_pipeline_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let render_path = BevyManifest::default().get_path("bevy_render");
    pipeline_key::derive_pipeline_key(input, render_path)
        .unwrap_or_else(|err| err.to_compile_error().into())
}

/// Above macro but for use in the `bevy_render` crate, separated due to path issues.
#[proc_macro_derive(
    PipelineKeyInRenderCrate,
    attributes(dynamic_key, not_fixed_size, custom_shader_defs)
)]
pub fn derive_pipeline_key_in_render_crate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let render_path = syn::Path::from(Crate(Span::call_site()));
    pipeline_key::derive_pipeline_key(input, render_path)
        .unwrap_or_else(|err| err.to_compile_error().into())
}
