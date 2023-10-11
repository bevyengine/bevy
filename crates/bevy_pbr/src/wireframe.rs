use crate::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath, TypeUuid};
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
///
/// Wireframes currently do not work when using webgl or webgpu.
/// Supported rendering backends:
/// - DX12
/// - Vulkan
/// - Metal
///
/// This is a native only feature.
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
            .register_type::<NoWireframe>()
            .register_type::<WireframeConfig>()
            .init_resource::<WireframeConfig>()
            .add_plugins(MaterialPlugin::<WireframeMaterial>::default())
            .add_systems(Startup, setup_global_wireframe_material)
            .add_systems(
                Update,
                (apply_global_wireframe_material, apply_wireframe_material),
            );
    }
}

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct Wireframe;

/// Disables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct NoWireframe;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes.
    /// Can be overridden for individual meshes by adding a [`Wireframe`] or [`NoWireframe`] component.
    pub global: bool,
}

#[derive(Resource)]
struct GlobalWireframeMaterial {
    // This handle will be reused when the global config is enabled
    handle: Handle<WireframeMaterial>,
}

fn setup_global_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
) {
    // Create the handle used for the global material
    commands.insert_resource(GlobalWireframeMaterial {
        handle: materials.add(WireframeMaterial {}),
    });
}

/// Applies or remove the wireframe material to any mesh with a [`Wireframe`] component.
fn apply_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    wireframes: Query<Entity, (With<Wireframe>, Without<Handle<WireframeMaterial>>)>,
    mut removed_wireframes: RemovedComponents<Wireframe>,
) {
    for e in removed_wireframes.read() {
        if let Some(mut commands) = commands.get_entity(e) {
            commands.remove::<Handle<WireframeMaterial>>();
        }
    }

    let mut wireframes_to_spawn = vec![];
    for e in &wireframes {
        wireframes_to_spawn.push((e, materials.add(WireframeMaterial {})));
    }
    commands.insert_or_spawn_batch(wireframes_to_spawn);
}

type WireframeFilter = (With<Handle<Mesh>>, Without<Wireframe>, Without<NoWireframe>);

/// Applies or removes a wireframe material on any mesh without a [`Wireframe`] component.
fn apply_global_wireframe_material(
    mut commands: Commands,
    config: Res<WireframeConfig>,
    meshes_without_material: Query<Entity, (WireframeFilter, Without<Handle<WireframeMaterial>>)>,
    meshes_with_global_material: Query<Entity, (WireframeFilter, With<Handle<WireframeMaterial>>)>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    if !config.is_changed() {
        return;
    }

    if config.global {
        let mut material_to_spawn = vec![];
        for e in &meshes_without_material {
            // We only add the material handle but not the Wireframe component
            // This makes it easy to detect which mesh is using the global material and which ones are user specified
            material_to_spawn.push((e, global_material.handle.clone()));
        }
        commands.insert_or_spawn_batch(material_to_spawn);
    } else if !config.global {
        for e in &meshes_with_global_material {
            commands.entity(e).remove::<Handle<WireframeMaterial>>();
        }
    }
}

#[derive(Default, AsBindGroup, TypeUuid, TypePath, Debug, Clone, Asset)]
#[uuid = "9e694f70-9963-4418-8bc1-3474c66b13b8"]
struct WireframeMaterial {}

impl Material for WireframeMaterial {
    fn fragment_shader() -> ShaderRef {
        WIREFRAME_SHADER_HANDLE.into()
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
