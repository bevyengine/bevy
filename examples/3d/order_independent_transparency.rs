//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    core_pipeline::oit::OrderIndependentTransparencySettings,
    prelude::*,
};
use bevy_render::view::RenderLayers;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_oit, cycle_scenes))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            // Add this component so this camera to render transparent meshes using OIT
            OrderIndependentTransparencySettings::default(),
            RenderLayers::layer(1),
        ))
        .insert(
            // Msaa currently doesn't work well with OIT
            Msaa::Off,
        );

    // light
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        },
        RenderLayers::layer(1),
    ));

    // spawn help text
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new("Press T to toggle OIT\n", TextStyle::default()),
            TextSection::new("OIT Enabled", TextStyle::default()),
            TextSection::new("\nPress C to cycle test scenes", TextStyle::default()),
        ]),
        RenderLayers::layer(1),
    ));

    // spawn default scene
    spawn_spheres(&mut commands, &mut meshes, &mut materials);
}

fn toggle_oit(
    mut commands: Commands,
    mut text: Query<&mut Text>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q: Query<(Entity, Has<OrderIndependentTransparencySettings>), With<Camera3d>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        let (e, has_oit) = q.single();
        text.single_mut().sections[1].value = if has_oit {
            // Removing the component will completely disable OIT for this camera
            commands
                .entity(e)
                .remove::<OrderIndependentTransparencySettings>();
            "OIT disabled".to_string()
        } else {
            // Adding the component to the camera will render any transparent meshes
            // with OIT instead of alpha blending
            commands
                .entity(e)
                .insert(OrderIndependentTransparencySettings::default());
            "OIT enabled".to_string()
        };
    }
}

fn cycle_scenes(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<Entity, With<Handle<StandardMaterial>>>,
    mut scene_id: Local<usize>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        // depsawn current scene
        for e in &q {
            commands.entity(e).despawn_recursive();
        }
        // increment scene_id
        *scene_id += 1;
        if *scene_id >= 2 {
            *scene_id = 0;
        }
        // spawn next scene
        match *scene_id {
            0 => spawn_occlusion_test(&mut commands, &mut meshes, &mut materials),
            1 => spawn_spheres(&mut commands, &mut meshes, &mut materials),
            _ => unreachable!(),
        }
    }
}

/// Spawns 3 overlapping spheres
/// Technically, when using `alpha_to_coverage` with MSAA this particular example wouldn't break,
/// but it breaks when disabling MSAA and is enough to show the difference between OIT enabled vs disabled.
fn spawn_spheres(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let pos_a = Vec3::new(-1.0, 0.75, 0.0);
    let pos_b = Vec3::new(0.0, -0.75, 0.0);
    let pos_c = Vec3::new(1.0, 0.75, 0.0);

    let offset = Vec3::new(0.0, 0.0, 0.0);

    let sphere_handle = meshes.add(Sphere::new(2.0).mesh());

    let alpha = 0.5;

    let render_layers = RenderLayers::layer(1);

    commands.spawn((
        PbrBundle {
            mesh: sphere_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: RED.with_alpha(alpha).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_translation(pos_a + offset),
            ..default()
        },
        render_layers.clone(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: GREEN.with_alpha(alpha).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_translation(pos_b + offset),
            ..default()
        },
        render_layers.clone(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere_handle,
            material: materials.add(StandardMaterial {
                base_color: BLUE.with_alpha(alpha).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_translation(pos_c + offset),
            ..default()
        },
        render_layers,
    ));
}

/// Spawn a combination of opaque cubes and transparent spheres.
/// This is useful to make sure transparent meshes drawn with OIT
/// are properly occluded by opaque meshes.
fn spawn_occlusion_test(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let sphere_handle = meshes.add(Sphere::new(1.0).mesh());
    let cube_handle = meshes.add(Cuboid::from_size(Vec3::ONE).mesh());
    let cube_material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    let render_layers = RenderLayers::layer(1);

    // front
    let x = -2.5;
    commands.spawn((
        PbrBundle {
            mesh: cube_handle.clone(),
            material: cube_material.clone(),
            transform: Transform::from_xyz(x, 0.0, 2.0),
            ..default()
        },
        render_layers.clone(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: RED.with_alpha(0.5).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_xyz(x, 0., 0.),
            ..default()
        },
        render_layers.clone(),
    ));

    // intersection
    commands.spawn((
        PbrBundle {
            mesh: cube_handle.clone(),
            material: cube_material.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        render_layers.clone(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: RED.with_alpha(0.5).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        render_layers.clone(),
    ));

    // back
    let x = 2.5;
    commands.spawn((
        PbrBundle {
            mesh: cube_handle.clone(),
            material: cube_material.clone(),
            transform: Transform::from_xyz(x, 0.0, -2.0),
            ..default()
        },
        render_layers.clone(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: RED.with_alpha(0.5).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_xyz(x, 0., 0.),
            ..default()
        },
        render_layers.clone(),
    ));
}
