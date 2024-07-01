use crate::{Material2d, Material2dKey, Material2dPlugin, Mesh2dHandle};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_color::{LinearRgba, Srgba};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath};
use bevy_render::{
    extract_resource::ExtractResource, mesh::MeshVertexBufferLayoutRef, prelude::*,
    render_resource::*,
};

pub const WIREFRAME_2D_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(6920362697190520314);

/// A [`Plugin`] that draws wireframes for 2D meshes.
///
/// Wireframes currently do not work when using webgl or webgpu.
/// Supported rendering backends:
/// - DX12
/// - Vulkan
/// - Metal
///
/// This is a native only feature.
#[derive(Debug, Default)]
pub struct Wireframe2dPlugin;
impl Plugin for Wireframe2dPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            WIREFRAME_2D_SHADER_HANDLE,
            "wireframe2d.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Wireframe2d>()
            .register_type::<NoWireframe2d>()
            .register_type::<Wireframe2dConfig>()
            .register_type::<Wireframe2dColor>()
            .init_resource::<Wireframe2dConfig>()
            .add_plugins(Material2dPlugin::<Wireframe2dMaterial>::default())
            .add_systems(Startup, setup_global_wireframe_material)
            .add_systems(
                Update,
                (
                    global_color_changed.run_if(resource_changed::<Wireframe2dConfig>),
                    wireframe_color_changed,
                    // Run `apply_global_wireframe_material` after `apply_wireframe_material` so that the global
                    // wireframe setting is applied to a mesh on the same frame its wireframe marker component is removed.
                    (apply_wireframe_material, apply_global_wireframe_material).chain(),
                ),
            );
    }
}

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`Wireframe2dConfig`] global setting.
///
/// This requires the [`Wireframe2dPlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct Wireframe2d;

/// Sets the color of the [`Wireframe2d`] of the entity it is attached to.
/// If this component is present but there's no [`Wireframe2d`] component,
/// it will still affect the color of the wireframe when [`Wireframe2dConfig::global`] is set to true.
///
/// This overrides the [`Wireframe2dConfig::default_color`].
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Wireframe2dColor {
    pub color: Srgba,
}

/// Disables wireframe rendering for any entity it is attached to.
/// It will ignore the [`Wireframe2dConfig`] global setting.
///
/// This requires the [`Wireframe2dPlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct NoWireframe2d;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct Wireframe2dConfig {
    /// Whether to show wireframes for all meshes.
    /// Can be overridden for individual meshes by adding a [`Wireframe2d`] or [`NoWireframe2d`] component.
    pub global: bool,
    /// If [`Self::global`] is set, any [`Entity`] that does not have a [`Wireframe2d`] component attached to it will have
    /// wireframes using this color. Otherwise, this will be the fallback color for any entity that has a [`Wireframe2d`],
    /// but no [`Wireframe2dColor`].
    pub default_color: Srgba,
}

#[derive(Resource)]
struct GlobalWireframe2dMaterial {
    // This handle will be reused when the global config is enabled
    handle: Handle<Wireframe2dMaterial>,
}

fn setup_global_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<Wireframe2dMaterial>>,
    config: Res<Wireframe2dConfig>,
) {
    // Create the handle used for the global material
    commands.insert_resource(GlobalWireframe2dMaterial {
        handle: materials.add(Wireframe2dMaterial {
            color: config.default_color.into(),
        }),
    });
}

/// Updates the wireframe material of all entities without a [`Wireframe2dColor`] or without a [`Wireframe2d`] component
fn global_color_changed(
    config: Res<Wireframe2dConfig>,
    mut materials: ResMut<Assets<Wireframe2dMaterial>>,
    global_material: Res<GlobalWireframe2dMaterial>,
) {
    if let Some(global_material) = materials.get_mut(&global_material.handle) {
        global_material.color = config.default_color.into();
    }
}

/// Updates the wireframe material when the color in [`Wireframe2dColor`] changes
#[allow(clippy::type_complexity)]
fn wireframe_color_changed(
    mut materials: ResMut<Assets<Wireframe2dMaterial>>,
    mut colors_changed: Query<
        (&mut Handle<Wireframe2dMaterial>, &Wireframe2dColor),
        (With<Wireframe2d>, Changed<Wireframe2dColor>),
    >,
) {
    for (mut handle, wireframe_color) in &mut colors_changed {
        *handle = materials.add(Wireframe2dMaterial {
            color: wireframe_color.color.into(),
        });
    }
}

/// Applies or remove the wireframe material to any mesh with a [`Wireframe2d`] component, and removes it
/// for any mesh with a [`NoWireframe2d`] component.
fn apply_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<Wireframe2dMaterial>>,
    wireframes: Query<
        (Entity, Option<&Wireframe2dColor>),
        (With<Wireframe2d>, Without<Handle<Wireframe2dMaterial>>),
    >,
    no_wireframes: Query<Entity, (With<NoWireframe2d>, With<Handle<Wireframe2dMaterial>>)>,
    mut removed_wireframes: RemovedComponents<Wireframe2d>,
    global_material: Res<GlobalWireframe2dMaterial>,
) {
    for e in removed_wireframes.read().chain(no_wireframes.iter()) {
        if let Some(mut commands) = commands.get_entity(e) {
            commands.remove::<Handle<Wireframe2dMaterial>>();
        }
    }

    let mut wireframes_to_spawn = vec![];
    for (e, wireframe_color) in &wireframes {
        let material = if let Some(wireframe_color) = wireframe_color {
            materials.add(Wireframe2dMaterial {
                color: wireframe_color.color.into(),
            })
        } else {
            // If there's no color specified we can use the global material since it's already set to use the default_color
            global_material.handle.clone()
        };
        wireframes_to_spawn.push((e, material));
    }
    commands.insert_or_spawn_batch(wireframes_to_spawn);
}

type Wireframe2dFilter = (
    With<Mesh2dHandle>,
    Without<Wireframe2d>,
    Without<NoWireframe2d>,
);

/// Applies or removes a wireframe material on any mesh without a [`Wireframe2d`] or [`NoWireframe2d`] component.
fn apply_global_wireframe_material(
    mut commands: Commands,
    config: Res<Wireframe2dConfig>,
    meshes_without_material: Query<
        Entity,
        (Wireframe2dFilter, Without<Handle<Wireframe2dMaterial>>),
    >,
    meshes_with_global_material: Query<
        Entity,
        (Wireframe2dFilter, With<Handle<Wireframe2dMaterial>>),
    >,
    global_material: Res<GlobalWireframe2dMaterial>,
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
            commands.entity(e).remove::<Handle<Wireframe2dMaterial>>();
        }
    }
}

#[derive(Default, AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct Wireframe2dMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for Wireframe2dMaterial {
    fn fragment_shader() -> ShaderRef {
        WIREFRAME_2D_SHADER_HANDLE.into()
    }

    fn depth_bias(&self) -> f32 {
        1.0
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}
