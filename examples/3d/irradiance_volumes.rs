use bevy::pbr::irradiance_volume::IrradianceVolume;
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
                [275.26923, 0.0, 0.0, 0.0],
                [0.0, 0.0, -275.26913, 0.0],
                [0.0, 275.26913, 0.0, 0.0],
                [-253.73419, 297.92352, -300.8109, 1.0],
            ])),
            ..SpatialBundle::default()
        })
        .insert(IrradianceVolume {
            voxels: asset_server.load::<Image>("irradiance_volumes/CornellBox.vxgi.ktx2"),
            intensity: 150.0,
        })
        .insert(LightProbe);
}
