use crate::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath, TypeUuid};
use bevy_render::{
    color::Color,
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
                (
                    global_color_changed.run_if(resource_changed::<WireframeConfig>()),
                    wireframe_color_changed,
                    apply_wireframe_material,
                    apply_global_wireframe_material.run_if(resource_changed::<WireframeConfig>()),
                ),
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

/// Sets the color of the [`Wireframe`] of the entity it is attached to.
/// If this component is present but there's no [`Wireframe`] component,
/// it will still affect the color of the wireframe when [`WireframeConfig::global`] is set to true.
///
/// This overrides the [`WireframeConfig::default_color`].
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct WireframeColor {
    pub color: Color,
}

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
            color: config.default_color,
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
        global_material.color = config.default_color;
    }
}

/// Updates the wireframe material when the color in [`WireframeColor`] changes
#[allow(clippy::type_complexity)]
fn wireframe_color_changed(
    mut materials: ResMut<Assets<WireframeMaterial>>,
    mut colors_changed: Query<
        (&mut Handle<WireframeMaterial>, &WireframeColor),
        (With<Wireframe>, Changed<WireframeColor>),
    >,
) {
    for (mut handle, wireframe_color) in &mut colors_changed {
        *handle = materials.add(WireframeMaterial {
            color: wireframe_color.color,
        });
    }
}

/// Applies or remove the wireframe material to any mesh with a [`Wireframe`] component.
fn apply_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    wireframes: Query<
        (Entity, Option<&WireframeColor>),
        (With<Wireframe>, Without<Handle<WireframeMaterial>>),
    >,
    mut removed_wireframes: RemovedComponents<Wireframe>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    for e in removed_wireframes.read() {
        if let Some(mut commands) = commands.get_entity(e) {
            commands.remove::<Handle<WireframeMaterial>>();
        }
    }

    let mut wireframes_to_spawn = vec![];
    for (e, wireframe_color) in &wireframes {
        let material = if let Some(wireframe_color) = wireframe_color {
            materials.add(WireframeMaterial {
                color: wireframe_color.color,
            })
        } else {
            // If there's no color specified we can use the global material since it's already set to use the default_color
            global_material.handle.clone()
        };
        wireframes_to_spawn.push((e, material));
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
    if config.global {
        let mut material_to_spawn = vec![];
        for e in &meshes_without_material {
            // We only add the material handle but not the Wireframe component
            // This makes it easy to detect which mesh is using the global material and which ones are user specified
            material_to_spawn.push((e, global_material.handle.clone()));
        }
        commands.insert_or_spawn_batch(material_to_spawn);
    } else {
        for e in &meshes_with_global_material {
            commands.entity(e).remove::<Handle<WireframeMaterial>>();
        }
    }
}

#[derive(Default, AsBindGroup, TypeUuid, TypePath, Debug, Clone, Asset)]
#[uuid = "9e694f70-9963-4418-8bc1-3474c66b13b8"]
pub struct WireframeMaterial {
    #[uniform(0)]
    pub color: Color,
}

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
