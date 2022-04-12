use bevy::prelude::*;
use example_utils::camera::{CameraController, CameraControllerPlugin};

fn main() {
    println!(
        "Controls:
    WSAD   - forward/back/strafe left/right
    LShift - 'run'
    E      - up
    Q      - down
    L      - switch between directional and point lights
    1/2    - decrease/increase point light depth bias
    3/4    - decrease/increase point light normal bias
    5/6    - decrease/increase direction light depth bias
    7/8    - decrease/increase direction light normal bias"
    );
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(CameraControllerPlugin)
        .add_startup_system(setup)
        .add_system(adjust_point_light_biases)
        .add_system(toggle_light)
        .add_system(adjust_directional_light_biases)
        .run();
}

/// set up a 3D scene to test shadow biases and perspective projections
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_plane_depth = 500.0f32;
    let spawn_height = 2.0;
    let sphere_radius = 0.25;

    let white_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..default()
    });
    let sphere_handle = meshes.add(Mesh::from(shape::Icosphere {
        radius: sphere_radius,
        ..default()
    }));

    println!("Using DirectionalLight");

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 0.0),
        point_light: PointLight {
            intensity: 0.0,
            range: spawn_plane_depth,
            color: Color::WHITE,
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    let theta = std::f32::consts::FRAC_PI_4;
    let light_transform = Mat4::from_euler(EulerRot::ZYX, 0.0, std::f32::consts::FRAC_PI_2, -theta);
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 100000.0,
            shadow_projection: OrthographicProjection {
                left: -0.35,
                right: 500.35,
                bottom: -0.1,
                top: 5.0,
                near: -5.0,
                far: 5.0,
                ..default()
            },
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_matrix(light_transform),
        ..default()
    });

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-1.0, 1.0, 1.0)
                .looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
            ..default()
        })
        .insert(CameraController::default());

    for z_i32 in -spawn_plane_depth as i32..=0 {
        commands.spawn_bundle(PbrBundle {
            mesh: sphere_handle.clone(),
            material: white_handle.clone(),
            transform: Transform::from_xyz(0.0, spawn_height, z_i32 as f32),
            ..default()
        });
    }

    // ground plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: 2.0 * spawn_plane_depth,
        })),
        material: white_handle,
        ..default()
    });
}

fn toggle_light(
    input: Res<Input<KeyCode>>,
    mut point_lights: Query<&mut PointLight>,
    mut directional_lights: Query<&mut DirectionalLight>,
) {
    if input.just_pressed(KeyCode::L) {
        for mut light in point_lights.iter_mut() {
            light.intensity = if light.intensity == 0.0 {
                println!("Using PointLight");
                100000000.0
            } else {
                0.0
            };
        }
        for mut light in directional_lights.iter_mut() {
            light.illuminance = if light.illuminance == 0.0 {
                println!("Using DirectionalLight");
                100000.0
            } else {
                0.0
            };
        }
    }
}

fn adjust_point_light_biases(input: Res<Input<KeyCode>>, mut query: Query<&mut PointLight>) {
    let depth_bias_step_size = 0.01;
    let normal_bias_step_size = 0.1;
    for mut light in query.iter_mut() {
        if input.just_pressed(KeyCode::Key1) {
            light.shadow_depth_bias -= depth_bias_step_size;
            println!("PointLight shadow_depth_bias: {}", light.shadow_depth_bias);
        }
        if input.just_pressed(KeyCode::Key2) {
            light.shadow_depth_bias += depth_bias_step_size;
            println!("PointLight shadow_depth_bias: {}", light.shadow_depth_bias);
        }
        if input.just_pressed(KeyCode::Key3) {
            light.shadow_normal_bias -= normal_bias_step_size;
            println!(
                "PointLight shadow_normal_bias: {}",
                light.shadow_normal_bias
            );
        }
        if input.just_pressed(KeyCode::Key4) {
            light.shadow_normal_bias += normal_bias_step_size;
            println!(
                "PointLight shadow_normal_bias: {}",
                light.shadow_normal_bias
            );
        }
    }
}

fn adjust_directional_light_biases(
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut DirectionalLight>,
) {
    let depth_bias_step_size = 0.01;
    let normal_bias_step_size = 0.1;
    for mut light in query.iter_mut() {
        if input.just_pressed(KeyCode::Key5) {
            light.shadow_depth_bias -= depth_bias_step_size;
            println!(
                "DirectionalLight shadow_depth_bias: {}",
                light.shadow_depth_bias
            );
        }
        if input.just_pressed(KeyCode::Key6) {
            light.shadow_depth_bias += depth_bias_step_size;
            println!(
                "DirectionalLight shadow_depth_bias: {}",
                light.shadow_depth_bias
            );
        }
        if input.just_pressed(KeyCode::Key7) {
            light.shadow_normal_bias -= normal_bias_step_size;
            println!(
                "DirectionalLight shadow_normal_bias: {}",
                light.shadow_normal_bias
            );
        }
        if input.just_pressed(KeyCode::Key8) {
            light.shadow_normal_bias += normal_bias_step_size;
            println!(
                "DirectionalLight shadow_normal_bias: {}",
                light.shadow_normal_bias
            );
        }
    }
}
