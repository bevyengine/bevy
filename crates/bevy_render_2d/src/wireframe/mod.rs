//! Wireframe for 2d meshes

mod components;
mod material;
mod resources;

use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{load_internal_asset, AssetApp, Assets};
use bevy_ecs::{
    entity::Entity,
    prelude::resource_changed,
    query::{Changed, With, Without},
    removal_detection::RemovedComponents,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{mesh::Mesh2d, render_resource::Shader};

use crate::{prelude::MeshMaterial2d, Material2dPlugin};

use material::WIREFRAME_2D_SHADER_HANDLE;
use resources::GlobalWireframe2dMaterial;
pub use {
    components::{NoWireframe2d, Wireframe2d, Wireframe2dColor},
    material::Wireframe2dMaterial,
    resources::Wireframe2dConfig,
};

type Wireframe2dFilter = (With<Mesh2d>, Without<Wireframe2d>, Without<NoWireframe2d>);

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
            .register_type::<Wireframe2dColor>()
            .init_resource::<Wireframe2dConfig>()
            .register_type::<Wireframe2dConfig>()
            .add_plugins(Material2dPlugin::<Wireframe2dMaterial>::default())
            .register_asset_reflect::<Wireframe2dMaterial>();

        app.add_systems(Startup, setup_global_wireframe_material)
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
    if let Some(global_material) = materials.get_mut(&*global_material) {
        global_material.color = config.default_color.into();
    }
}

/// Updates the wireframe material when the color in [`Wireframe2dColor`] changes
fn wireframe_color_changed(
    mut materials: ResMut<Assets<Wireframe2dMaterial>>,
    mut colors_changed: Query<
        (&mut MeshMaterial2d<Wireframe2dMaterial>, &Wireframe2dColor),
        (With<Wireframe2d>, Changed<Wireframe2dColor>),
    >,
) {
    for (mut handle, wireframe_color) in &mut colors_changed {
        handle.0 = materials.add(Wireframe2dMaterial {
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
        (
            With<Wireframe2d>,
            Without<MeshMaterial2d<Wireframe2dMaterial>>,
        ),
    >,
    no_wireframes: Query<
        Entity,
        (
            With<NoWireframe2d>,
            With<MeshMaterial2d<Wireframe2dMaterial>>,
        ),
    >,
    mut removed_wireframes: RemovedComponents<Wireframe2d>,
    global_material: Res<GlobalWireframe2dMaterial>,
) {
    for e in removed_wireframes.read().chain(no_wireframes.iter()) {
        if let Ok(mut commands) = commands.get_entity(e) {
            commands.remove::<MeshMaterial2d<Wireframe2dMaterial>>();
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
            global_material.handle()
        };
        wireframes_to_spawn.push((e, MeshMaterial2d(material)));
    }
    commands.try_insert_batch(wireframes_to_spawn);
}

/// Applies or removes a wireframe material on any mesh without a [`Wireframe2d`] or [`NoWireframe2d`] component.
fn apply_global_wireframe_material(
    mut commands: Commands,
    config: Res<Wireframe2dConfig>,
    meshes_without_material: Query<
        Entity,
        (
            Wireframe2dFilter,
            Without<MeshMaterial2d<Wireframe2dMaterial>>,
        ),
    >,
    meshes_with_global_material: Query<
        Entity,
        (Wireframe2dFilter, With<MeshMaterial2d<Wireframe2dMaterial>>),
    >,
    global_material: Res<GlobalWireframe2dMaterial>,
) {
    if config.global {
        let mut material_to_spawn = vec![];
        for e in &meshes_without_material {
            // We only add the material handle but not the Wireframe component
            // This makes it easy to detect which mesh is using the global material and which ones are user specified
            material_to_spawn.push((e, MeshMaterial2d(global_material.handle())));
        }
        commands.try_insert_batch(material_to_spawn);
    } else {
        for e in &meshes_with_global_material {
            commands
                .entity(e)
                .remove::<MeshMaterial2d<Wireframe2dMaterial>>();
        }
    }
}
