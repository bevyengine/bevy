//! Demonstrates how to prevent meshes from casting/receiving shadows in a 3d scene.

use std::f32::consts::PI;

use bevy::{
    color::palettes::basic::{BLUE, LIME, RED},
    light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

fn main() {
    println!(
        "Controls:
    C      - toggle shadow casters (i.e. casters become not, and not casters become casters)
    R      - toggle shadow receivers (i.e. receivers become not, and not receivers become receivers)
    L      - switch between directional and point lights"
    );
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_light, toggle_shadows))
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
    let sphere_handle = meshes.add(Sphere::new(sphere_radius));

    // sphere - initially a caster
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(Color::from(RED))),
        Transform::from_xyz(-1.0, spawn_height, 0.0),
    ));

    // sphere - initially not a caster
    commands.spawn((
        Mesh3d(sphere_handle),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(1.0, spawn_height, 0.0),
        NotShadowCaster,
    ));

    // floating plane - initially not a shadow receiver and not a caster
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::from(LIME))),
        Transform::from_xyz(0.0, 1.0, -10.0),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // lower ground plane - initially a shadow receiver
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(white_handle),
    ));

    println!("Using DirectionalLight");

    commands.spawn((
        PointLight {
            intensity: 0.0,
            range: spawn_plane_depth,
            color: Color::WHITE,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 5.0, 0.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 2., -PI / 4.)),
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 7.0,
            maximum_distance: 25.0,
            ..default()
        }
        .build(),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-5.0, 5.0, 5.0).looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
    ));
}

fn toggle_light(
    input: Res<ButtonInput<KeyCode>>,
    mut point_lights: Query<&mut PointLight>,
    mut directional_lights: Query<&mut DirectionalLight>,
) {
    if input.just_pressed(KeyCode::KeyL) {
        for mut light in &mut point_lights {
            light.intensity = if light.intensity == 0.0 {
                println!("Using PointLight");
                1_000_000.0 // Mini-sun point light
            } else {
                0.0
            };
        }
        for mut light in &mut directional_lights {
            light.illuminance = if light.illuminance == 0.0 {
                println!("Using DirectionalLight");
                light_consts::lux::OVERCAST_DAY
            } else {
                0.0
            };
        }
    }
}

fn toggle_shadows(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut queries: ParamSet<(
        Query<Entity, (With<Mesh3d>, With<NotShadowCaster>)>,
        Query<Entity, (With<Mesh3d>, With<NotShadowReceiver>)>,
        Query<Entity, (With<Mesh3d>, Without<NotShadowCaster>)>,
        Query<Entity, (With<Mesh3d>, Without<NotShadowReceiver>)>,
    )>,
) {
    if input.just_pressed(KeyCode::KeyC) {
        println!("Toggling casters");
        for entity in queries.p0().iter() {
            commands.entity(entity).remove::<NotShadowCaster>();
        }
        for entity in queries.p2().iter() {
            commands.entity(entity).insert(NotShadowCaster);
        }
    }
    if input.just_pressed(KeyCode::KeyR) {
        println!("Toggling receivers");
        for entity in queries.p1().iter() {
            commands.entity(entity).remove::<NotShadowReceiver>();
        }
        for entity in queries.p3().iter() {
            commands.entity(entity).insert(NotShadowReceiver);
        }
    }
}
