//! Update a scene from a glTF file, either by spawning the scene as a child of another entity,
//! or by accessing the entities of the scene.

use bevy::{light::DirectionalLightShadowMap, prelude::*};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, move_scene_entities)
        .run();
}

#[derive(Component)]
struct MovedScene;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Transform::from_xyz(4.0, 25.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-0.5, 0.9, 1.5).looking_at(Vec3::new(-0.5, 0.3, 0.0), Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
            ..default()
        },
    ));

    // Spawn the scene as a child of this entity at the given transform
    commands.spawn((
        Transform::from_xyz(-1.0, 0.0, 0.0),
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
    ));

    // Spawn a second scene, and add a tag component to be able to target it later
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        MovedScene,
    ));
}

// This system will move all entities that are descendants of MovedScene (which will be all entities spawned in the scene)
fn move_scene_entities(
    time: Res<Time>,
    moved_scene: Query<Entity, With<MovedScene>>,
    children: Query<&Children>,
    mut transforms: Query<&mut Transform>,
) {
    for moved_scene_entity in &moved_scene {
        let mut offset = 0.;
        for entity in children.iter_descendants(moved_scene_entity) {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = Vec3::new(
                    offset * ops::sin(time.elapsed_secs()) / 20.,
                    0.,
                    ops::cos(time.elapsed_secs()) / 20.,
                );
                offset += 0.5;
            }
        }
    }
}
