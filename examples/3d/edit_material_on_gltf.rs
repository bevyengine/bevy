//! Showcases how to change the material of a `Scene` spawned from a Gltf

use bevy::{
    app::{App, PluginGroup, Startup},
    asset::{AssetServer, Assets},
    audio::AudioPlugin,
    color::{palettes, Color},
    gltf::GltfAssetLabel,
    light::DirectionalLight,
    math::{Dir3, Vec3},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{Camera3d, Children, Commands, Component, On, Query, Res, ResMut, Transform},
    scene::{SceneInstanceReady, SceneRoot},
    DefaultPlugins,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<AudioPlugin>())
        .add_systems(Startup, setup_scene)
        .add_observer(change_material)
        .run();
}

/// This is added to a [`SceneRoot`] and will cause the [`StandardMaterial::base_color`]
/// of all materials to be overwritten
#[derive(Component)]
struct ColorOverride(Color);

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1., 2.5).looking_at(Vec3::new(0., 0.25, 0.), Dir3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(0., 1., 0.25).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    // FlightHelmet handle
    let flight_helmet = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    // This model will keep its original materials
    commands.spawn(SceneRoot(flight_helmet.clone()));
    // This model will be tinted red
    commands.spawn((
        SceneRoot(flight_helmet.clone()),
        Transform::from_xyz(-1.25, 0., 0.),
        ColorOverride(palettes::tailwind::RED_300.into()),
    ));
    // This model will be tinted green
    commands.spawn((
        SceneRoot(flight_helmet),
        Transform::from_xyz(1.25, 0., 0.),
        ColorOverride(palettes::tailwind::GREEN_300.into()),
    ));
}

fn change_material(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    color_override: Query<&ColorOverride>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut asset_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Get the `ColorOverride` of the entity, if it does not have a color override, skip
    let Ok(color_override) = color_override.get(trigger.target()) else {
        return;
    };

    // Iterate over all children recursively
    for descendants in children.iter_descendants(trigger.target()) {
        // Get the material of the descendant
        if let Some(material) = mesh_materials
            .get(descendants)
            .ok()
            .and_then(|id| asset_materials.get_mut(id.id()))
        {
            // Create a copy of the material and override base color
            // If you intend on creating multiple models with the same tint, it
            // is best to cache the handle somewhere, as having multiple materials
            // that are identical is expensive
            let mut new_material = material.clone();
            new_material.base_color = color_override.0;

            // Override `MeshMaterial3d` with new material
            commands
                .entity(descendants)
                .insert(MeshMaterial3d(asset_materials.add(new_material)));
        }
    }
}
