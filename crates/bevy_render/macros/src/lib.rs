#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod as_bind_group;
mod extract_component;
mod extract_resource;
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

/// Derive macro for the `ExtractResource` trait.
///
/// Extracts a `Resource` from the main world into the render world by cloning it
/// each frame during the `ExtractSchedule`. The type must implement `Clone`.
///
/// See the `ExtractResource` trait docs for full explanation.
///
/// ```ignore
/// #[derive(Resource, Clone, ExtractResource)]
/// struct MyRenderSettings {
///     clear_color: Color,
/// }
/// ```
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

/// Derive macro for the `AsBindGroup` trait.
///
/// Converts a type into a `BindGroup` for use in shaders. Field attributes
/// define which fields become GPU bindings.
///
/// See the `AsBindGroup` trait docs for full explanation.
///
/// # Field attributes
///
/// ```ignore
/// #[derive(AsBindGroup)]
/// struct MyMaterial {
///     // Bind a field as a uniform buffer (must implement ShaderType).
///     #[uniform(0)]
///     color: LinearRgba,
///
///     // Bind as a texture and sampler (field must be Handle<Image> or Option<Handle<Image>>).
///     #[texture(1)]
///     #[sampler(2)]
///     color_texture: Handle<Image>,
///
///     // Bind as a storage buffer (Handle<ShaderBuffer> or raw Buffer with `buffer` flag).
///     #[storage(3, read_only)]
///     values: Handle<ShaderBuffer>,
///     #[storage(4, read_only, buffer)]
///     raw_buf: Buffer,
///
///     // Bind as a storage texture.
///     #[storage_texture(5)]
///     output: Handle<Image>,
/// }
/// ```
///
/// ## `texture` arguments
///
/// | Argument              | Values                                                         | Default              |
/// |-----------------------|----------------------------------------------------------------|----------------------|
/// | `dimension` = "..."   | `"1d"`, `"2d"`, `"2d_array"`, `"3d"`, `"cube"`, `"cube_array"` | `"2d"`               |
/// | `sample_type` = "..." | `"float"`, `"depth"`, `"s_int"`, `"u_int"`                     | `"float"`            |
/// | `filterable` = ...    | `true`, `false`                                                | `true`               |
/// | `multisampled` = ...  | `true`, `false`                                                | `false`              |
/// | `visibility(...)`     | `all`, `none`, or a list of `vertex`, `fragment`, `compute`    | `vertex`, `fragment` |
///
/// ## `sampler` arguments
///
/// | Argument               | Values                                            | Default              |
/// |------------------------|---------------------------------------------------|----------------------|
/// | `sampler_type` = "..." | `"filtering"`, `"non_filtering"`, `"comparison"`  | `"filtering"`        |
/// | `visibility(...)`      | `all`, `none`, or a list of `vertex`, `fragment`, `compute` | `vertex`, `fragment` |
///
/// ## `storage` arguments
///
/// | Argument            | Values                                                      | Default              |
/// |---------------------|-------------------------------------------------------------|----------------------|
/// | `read_only`         | if present, buffer is read-only                             | `false`              |
/// | `buffer`            | if present, field is a raw wgpu `Buffer`                    |                      |
/// | `visibility(...)`   | `all`, `none`, or a list of `vertex`, `fragment`, `compute` | `vertex`, `fragment` |
///
/// ## `storage_texture` arguments
///
/// | Argument             | Values                                                      | Default      |
/// |----------------------|-------------------------------------------------------------|--------------|
/// | `dimension` = "..."  | `"1d"`, `"2d"`, `"2d_array"`, `"3d"`, `"cube"`, `"cube_array"` | `"2d"`   |
/// | `image_format` = ... | any `TextureFormat` member                                  | `Rgba8Unorm` |
/// | `access` = ...       | any `StorageTextureAccess` member                           | `ReadWrite`  |
/// | `visibility(...)`    | `all`, `none`, or a list of `vertex`, `fragment`, `compute` | `compute`    |
///
/// # Struct-level attributes
///
/// ```ignore
/// // Convert the whole struct to a shader type for uniform binding.
/// #[derive(AsBindGroup)]
/// #[uniform(0, MyMaterialUniform)]
/// struct MyMaterial { /* ... */ }
///
/// // Store extra data alongside the bind group for pipeline specialization.
/// #[derive(AsBindGroup)]
/// #[bind_group_data(MyMaterialKey)]
/// struct MyMaterial { /* ... */ }
///
/// // Enable bindless mode for reduced GPU state changes.
/// #[derive(AsBindGroup)]
/// #[bindless]
/// struct MyMaterial { /* ... */ }
///
/// // Use `data` for a single buffer containing an array (instead of array of buffers).
/// #[derive(AsBindGroup)]
/// #[data(0, MyMaterialUniform, binding_array(10))]
/// struct MyMaterial { /* ... */ }
/// ```
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
