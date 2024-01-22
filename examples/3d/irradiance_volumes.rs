use bevy::pbr::irradiance_volume::{IrradianceVolume, IrradianceVoxels};
use bevy::prelude::*;

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

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-278.0, 273.0, 800.0),
        ..default()
    });

    commands
        .spawn(SpatialBundle {
            transform: Transform::from_matrix(Mat4::from_cols_array_2d(&[
                [45.878204, 0.0, 0.0, 0.0],
                [0.0, 0.0, -45.87819, 0.0],
                [0.0, 45.87819, 0.0, 0.0],
                [-529.0034, 22.654373, -25.541794, 1.0],
            ])),
            ..SpatialBundle::default()
        })
        .insert(IrradianceVolume {
            voxels: asset_server.load::<IrradianceVoxels>("irradiance_volumes/CornellBox.vxgi"),
            intensity: 150.0,
        });
}
