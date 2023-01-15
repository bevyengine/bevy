use crate::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypeUuid};
use bevy_render::{
    extract_resource::ExtractResource,
    mesh::{Mesh, MeshVertexBufferLayout},
    prelude::Shader,
    render_resource::{
        AsBindGroup, PolygonMode, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
    },
};

pub const WIREFRAME_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(192598014480025766);

/// A [`Plugin`] that draws wireframes.
#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            WIREFRAME_SHADER_HANDLE,
            "render/wireframe.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Wireframe>()
            .register_type::<WireframeConfig>()
            .init_resource::<WireframeConfig>()
            .add_plugin(MaterialPlugin::<WireframeMaterial>::default())
            .add_system(apply_global)
            .add_system(apply_material);
    }

/// Toggles wireframe rendering for any entity it is attached to.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Wireframe;

/// Configuration resource for [`WireframePlugin`].
#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes.
    /// If `false`, only meshes with a [`Wireframe`] component will be rendered.
    pub global: bool,
}

/// Applies the wireframe material to any mesh with a [`Wireframe`] component.
fn apply_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    wireframes: Query<Entity, (With<Wireframe>, Without<Handle<WireframeMaterial>>)>,
) {
    for e in &wireframes {
        commands
            .entity(e)
            .insert(materials.add(WireframeMaterial {}));
    }
}

/// Applies or removes a wireframe material on any mesh without a [`Wireframe`] component.
#[allow(clippy::type_complexity)]
fn apply_global(
    mut commands: Commands,
    config: Res<WireframeConfig>,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    meshes_without_material: Query<
        Entity,
        (
            With<Handle<Mesh>>,
            Without<Wireframe>,
            Without<Handle<WireframeMaterial>>,
        ),
    >,
    meshes_with_material: Query<
        Entity,
        (
            With<Handle<Mesh>>,
            Without<Wireframe>,
            With<Handle<WireframeMaterial>>,
        ),
    >,
) {
    if !config.is_changed() {
        return;
    }

    if config.global {
        let global_material = materials.add(WireframeMaterial {});
        for e in &meshes_without_material {
            commands.entity(e).insert(global_material.clone());
        }
    } else if !config.global {
        for e in &meshes_with_material {
            commands.entity(e).remove::<Handle<WireframeMaterial>>();
        }
    }
}

#[derive(Default, AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "9e694f70-9963-4418-8bc1-3474c66b13b8"]
struct WireframeMaterial {}

impl Material for WireframeMaterial {
    fn vertex_shader() -> ShaderRef {
        WIREFRAME_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        WIREFRAME_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        Ok(())
    }
}
