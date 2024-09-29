use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Asset, AssetApp, Assets, Handle, ReflectAsset};
use bevy_color::{Color, ColorToComponents};
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_math::Vec4;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    render_asset::{prepare_assets, RenderAssetPlugin, RenderAssets},
    render_phase::AddRenderCommand,
    render_resource::{
        AsBindGroup, AsBindGroupShaderType, Shader, ShaderRef, ShaderType, SpecializedMeshPipelines,
    },
    texture::GpuImage,
    view::ViewVisibility,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

use crate::Mesh2d;

use super::{
    queue_material2d_meshes, DrawMaterial2d, HasMaterial2d, Material2d, Material2dPipeline,
    PreparedMaterial2d, RenderMaterial2dInstances,
};

pub const PLACEHOLDER_MATERIAL_2D_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8613664800973594341);

/// A plugin for setting up [`PlaceholderMaterial2d`] and applying it to every [`Mesh2d`] with no [`MeshMaterial2d`].
///
/// [`MeshMaterial2d`]: crate::MeshMaterial2d
#[derive(Default)]
pub struct PlaceholderMaterial2dPlugin;

impl Plugin for PlaceholderMaterial2dPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PLACEHOLDER_MATERIAL_2D_SHADER_HANDLE,
            "placeholder_material.wgsl",
            Shader::from_wgsl
        );

        app.init_asset::<PlaceholderMaterial2d>()
            .register_asset_reflect::<PlaceholderMaterial2d>()
            .add_plugins(RenderAssetPlugin::<PreparedMaterial2d<PlaceholderMaterial2d>>::default());

        app.world_mut()
            .resource_mut::<Assets<PlaceholderMaterial2d>>()
            .insert(
                &Handle::<PlaceholderMaterial2d>::default(),
                PlaceholderMaterial2d::from(Color::srgb(1.0, 0.0, 1.0)),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque2d, DrawMaterial2d<PlaceholderMaterial2d>>()
                .add_render_command::<AlphaMask2d, DrawMaterial2d<PlaceholderMaterial2d>>()
                .add_render_command::<Transparent2d, DrawMaterial2d<PlaceholderMaterial2d>>()
                .init_resource::<RenderMaterial2dInstances<PlaceholderMaterial2d>>()
                .init_resource::<SpecializedMeshPipelines<Material2dPipeline<PlaceholderMaterial2d>>>()
                .add_systems(ExtractSchedule, extract_placeholder_material_meshes_2d)
                .add_systems(
                    Render,
                    queue_material2d_meshes::<PlaceholderMaterial2d>
                        .in_set(RenderSet::QueueMeshes)
                        .after(prepare_assets::<PreparedMaterial2d<PlaceholderMaterial2d>>),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<Material2dPipeline<PlaceholderMaterial2d>>();
        }
    }
}

/// A placeholder material used for [`Mesh2d`] entities without a [`MeshMaterial2d`].
///
/// By default, this renders a white material. The color can be overridden by inserting a custom
/// material for the default asset handle.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
///
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<PlaceholderMaterial2d>>,
/// ) {
///     // Optional: Insert a custom placeholder material.
///     materials.insert(
///         Handle::<PlaceholderMaterial2d>::default(),
///         PlaceholderMaterial2d::from(Color::from_srgb(1.0, 0.0, 1.0)),
///     );
///
///     // Spawn a circle with no material.
///     // The mesh will be rendered with the placeholder material.
///     commands.spawn(Mesh2d(meshes.add(Circle::new(50.0))));
/// }
/// ```
///
/// [`MeshMaterial2d`]: crate::MeshMaterial2d
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[reflect(Asset, Debug, Default)]
#[uniform(0, PlaceholderMaterial2dUniform)]
pub struct PlaceholderMaterial2d {
    /// The color of the material.
    pub color: Color,
}

impl Default for PlaceholderMaterial2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
        }
    }
}

impl Material2d for PlaceholderMaterial2d {
    fn fragment_shader() -> ShaderRef {
        PLACEHOLDER_MATERIAL_2D_SHADER_HANDLE.into()
    }
}

/// The GPU representation of the uniform data of a [`PlaceholderMaterial2d`].
#[derive(Clone, Default, ShaderType)]
pub struct PlaceholderMaterial2dUniform {
    /// The color of the material.
    pub color: Vec4,
}

impl AsBindGroupShaderType<PlaceholderMaterial2dUniform> for PlaceholderMaterial2d {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<GpuImage>,
    ) -> PlaceholderMaterial2dUniform {
        PlaceholderMaterial2dUniform {
            color: self.color.to_linear().to_vec4(),
        }
    }
}

impl From<Color> for PlaceholderMaterial2d {
    fn from(color: Color) -> Self {
        Self { color }
    }
}

pub(crate) fn extract_placeholder_material_meshes_2d(
    mut material_instances: ResMut<RenderMaterial2dInstances<PlaceholderMaterial2d>>,
    query: Extract<Query<(Entity, &ViewVisibility), (With<Mesh2d>, Without<HasMaterial2d>)>>,
) {
    material_instances.clear();

    for (entity, view_visibility) in &query {
        if view_visibility.get() {
            material_instances.insert(entity, Handle::<PlaceholderMaterial2d>::default().id());
        }
    }
}
