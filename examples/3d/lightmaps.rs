//! Rendering a scene with baked lightmaps.

use bevy::pbr::Lightmap;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, add_lightmaps_to_meshes)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("models/CornellBox/CornellBox.glb")),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-278.0, 273.0, 800.0),
        ..default()
    });
}

fn add_lightmaps_to_meshes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<
        (Entity, &Name, &Handle<StandardMaterial>),
        (With<Handle<Mesh>>, Without<Lightmap>),
    >,
) {
    let exposure = 250.0;
    for (entity, name, material) in meshes.iter() {
        if &**name == "large_box" {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Large.zstd.ktx2"),
                ..default()
            });
            continue;
        }

        if &**name == "small_box" {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Small.zstd.ktx2"),
                ..default()
            });
            continue;
        }

        if name.starts_with("cornell_box") {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Box.zstd.ktx2"),
                ..default()
            });
            continue;
        }
    }
}
