use bevy::prelude::*;

/// This example shows how to configure Multi-Sample Anti-Aliasing. Setting the sample count higher
/// will result in smoother edges, but it will also increase the cost to render those edges. The
/// range should generally be somewhere between 1 (no multi sampling, but cheap) to 8 (crisp but
/// expensive).
/// Note that WGPU currently only supports 1 or 4 samples.
/// Ultimately we plan on supporting whatever is natively supported on a given device.
/// Check out this issue for more info: https://github.com/gfx-rs/wgpu/issues/1832
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(cycle_msaa)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Press 'm' to toggle MSAA");
    info!("Using 4x MSAA");

    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-3.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn cycle_msaa(input: Res<Input<KeyCode>>, mut msaa: ResMut<Msaa>) {
    if input.just_pressed(KeyCode::M) {
        if msaa.samples == 4 {
            info!("Not using MSAA");
            msaa.samples = 1;
        } else {
            info!("Using 4x MSAA");
            msaa.samples = 4;
        }
    }
}
