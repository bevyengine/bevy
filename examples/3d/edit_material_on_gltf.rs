//! Showcases how to change the material of a `Scene` spawned from a Gltf

use bevy::{
    app::{App, PluginGroup, Startup},
    asset::{AssetServer, Assets},
    audio::AudioPlugin,
    color::{palettes, Color},
    gltf::GltfAssetLabel,
    math::{Dir3, Vec3},
    pbr::{DirectionalLight, MeshMaterial3d, StandardMaterial},
    prelude::{Camera3d, Children, Commands, Component, Query, Res, ResMut, Transform, Trigger},
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

/// Overrides the color of the `StandardMaterial` of a mesh
#[derive(Component)]
struct ColorOverride(Color);

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1., 2.5).looking_at(Vec3::new(0., 0.25, 0.), Dir3::Y),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(0., 1., 0.25).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    // Flight Helmets
    commands.spawn((SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
    )),));
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        Transform::from_xyz(-1.25, 0., 0.),
        ColorOverride(palettes::tailwind::RED_300.into()),
    ));
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        Transform::from_xyz(1.25, 0., 0.),
        ColorOverride(palettes::tailwind::GREEN_300.into()),
    ));
}

fn change_material(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    color_override: Query<&ColorOverride>,
    materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(color_override) = color_override.get(trigger.target()) else {
        return;
    };

    for child in children.iter_descendants(trigger.target()) {
        if let Some(material) = materials
            .get(child)
            .ok()
            .and_then(|id| standard_materials.get_mut(id.id()))
        {
            let mut new_material = material.clone();
            new_material.base_color = color_override.0;

            commands
                .entity(child)
                .insert(MeshMaterial3d(standard_materials.add(new_material)));
        }
    }
}
