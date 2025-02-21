use crate::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{load_internal_asset, weak_handle, Asset, AssetApp, Assets, Handle};
use bevy_color::{Color, LinearRgba};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_resource::ExtractResource,
    mesh::{Mesh3d, MeshVertexBufferLayoutRef},
    prelude::*,
    render_resource::*,
};

pub const WIREFRAME_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("2646a633-f8e3-4380-87ae-b44d881abbce");

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
            .register_type::<WireframeColor>()
            .init_resource::<WireframeConfig>()
            .add_plugins(MaterialPlugin::<WireframeMaterial>::default())
            .register_asset_reflect::<WireframeMaterial>()
            .add_systems(Startup, setup_global_wireframe_material)
            .add_systems(
                Update,
                (
                    global_color_changed.run_if(resource_changed::<WireframeConfig>),
                    wireframe_color_changed,
                    // Run `apply_global_wireframe_material` after `apply_wireframe_material` so that the global
                    // wireframe setting is applied to a mesh on the same frame its wireframe marker component is removed.
                    (apply_wireframe_material, apply_global_wireframe_material).chain(),
                ),
            );
    }
}

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Wireframe;

/// Sets the color of the [`Wireframe`] of the entity it is attached to.
///
/// If this component is present but there's no [`Wireframe`] component,
/// it will still affect the color of the wireframe when [`WireframeConfig::global`] is set to true.
///
/// This overrides the [`WireframeConfig::default_color`].
// TODO: consider caching materials based on this color.
// This could blow up in size if people use random colored wireframes for each mesh.
// It will also be important to remove unused materials from the cache.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct WireframeColor {
    pub color: Color,
}

/// Disables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct NoWireframe;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes.
    /// Can be overridden for individual meshes by adding a [`Wireframe`] or [`NoWireframe`] component.
    pub global: bool,
    /// If [`Self::global`] is set, any [`Entity`] that does not have a [`Wireframe`] component attached to it will have
    /// wireframes using this color. Otherwise, this will be the fallback color for any entity that has a [`Wireframe`],
    /// but no [`WireframeColor`].
    pub default_color: Color,
}

#[derive(Resource)]
struct GlobalWireframeMaterial {
    // This handle will be reused when the global config is enabled
    handle: Handle<WireframeMaterial>,
}

fn setup_global_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    config: Res<WireframeConfig>,
) {
    // Create the handle used for the global material
    commands.insert_resource(GlobalWireframeMaterial {
        handle: materials.add(WireframeMaterial {
            color: config.default_color.into(),
        }),
    });
}

/// Updates the wireframe material of all entities without a [`WireframeColor`] or without a [`Wireframe`] component
fn global_color_changed(
    config: Res<WireframeConfig>,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    if let Some(global_material) = materials.get_mut(&global_material.handle) {
        global_material.color = config.default_color.into();
    }
}

/// Updates the wireframe material when the color in [`WireframeColor`] changes
fn wireframe_color_changed(
    mut materials: ResMut<Assets<WireframeMaterial>>,
    mut colors_changed: Query<
        (&mut MeshMaterial3d<WireframeMaterial>, &WireframeColor),
        (With<Wireframe>, Changed<WireframeColor>),
    >,
) {
    for (mut handle, wireframe_color) in &mut colors_changed {
        handle.0 = materials.add(WireframeMaterial {
            color: wireframe_color.color.into(),
        });
    }
}

/// Applies or remove the wireframe material to any mesh with a [`Wireframe`] component, and removes it
/// for any mesh with a [`NoWireframe`] component.
fn apply_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    wireframes: Query<
        (Entity, Option<&WireframeColor>),
        (With<Wireframe>, Without<MeshMaterial3d<WireframeMaterial>>),
    >,
    no_wireframes: Query<Entity, (With<NoWireframe>, With<MeshMaterial3d<WireframeMaterial>>)>,
    mut removed_wireframes: RemovedComponents<Wireframe>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    for e in removed_wireframes.read().chain(no_wireframes.iter()) {
        if let Some(mut commands) = commands.get_entity(e) {
            commands.remove::<MeshMaterial3d<WireframeMaterial>>();
        }
    }

    let mut material_to_spawn = vec![];
    for (e, maybe_color) in &wireframes {
        let material = get_wireframe_material(maybe_color, &mut materials, &global_material);
        material_to_spawn.push((e, MeshMaterial3d(material)));
    }
    commands.insert_or_spawn_batch(material_to_spawn);
}

type WireframeFilter = (With<Mesh3d>, Without<Wireframe>, Without<NoWireframe>);

/// Applies or removes a wireframe material on any mesh without a [`Wireframe`] or [`NoWireframe`] component.
fn apply_global_wireframe_material(
    mut commands: Commands,
    config: Res<WireframeConfig>,
    meshes_without_material: Query<
        (Entity, Option<&WireframeColor>),
        (WireframeFilter, Without<MeshMaterial3d<WireframeMaterial>>),
    >,
    meshes_with_global_material: Query<
        Entity,
        (WireframeFilter, With<MeshMaterial3d<WireframeMaterial>>),
    >,
    global_material: Res<GlobalWireframeMaterial>,
    mut materials: ResMut<Assets<WireframeMaterial>>,
) {
    if config.global {
        let mut material_to_spawn = vec![];
        for (e, maybe_color) in &meshes_without_material {
            let material = get_wireframe_material(maybe_color, &mut materials, &global_material);
            // We only add the material handle but not the Wireframe component
            // This makes it easy to detect which mesh is using the global material and which ones are user specified
            material_to_spawn.push((e, MeshMaterial3d(material)));
        }
        commands.insert_or_spawn_batch(material_to_spawn);
    } else {
        for e in &meshes_with_global_material {
            commands
                .entity(e)
                .remove::<MeshMaterial3d<WireframeMaterial>>();
        }
    }
}

/// Gets a handle to a wireframe material with a fallback on the default material
fn get_wireframe_material(
    maybe_color: Option<&WireframeColor>,
    wireframe_materials: &mut Assets<WireframeMaterial>,
    global_material: &GlobalWireframeMaterial,
) -> Handle<WireframeMaterial> {
    if let Some(wireframe_color) = maybe_color {
        wireframe_materials.add(WireframeMaterial {
            color: wireframe_color.color.into(),
        })
    } else {
        // If there's no color specified we can use the global material since it's already set to use the default_color
        global_material.handle.clone()
    }
}

#[derive(Default, AsBindGroup, Debug, Clone, Asset, Reflect)]
pub struct WireframeMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material for WireframeMaterial {
    fn fragment_shader() -> ShaderRef {
        WIREFRAME_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.bias.slope_scale = 1.0;
        }
        Ok(())
    }
}
