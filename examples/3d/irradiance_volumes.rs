use bevy::prelude::*;
use bevy_internal::pbr::IrradianceVolume;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/CornellBox/CornellBox.glb#Scene0"),
        ..SceneBundle::default()
    });

    commands
        .spawn(SpatialBundle {
            transform: Transform::IDENTITY,
            ..SpatialBundle::default()
        })
        .insert(
            asset_server.load::<IrradianceVolume>("irradiance_volumes/CornellBox.voxelgi.bincode"),
        );
}
