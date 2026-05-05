//! A simple 3D scene showing how alpha blending can break and how order independent transparency (OIT) can fix it.
//!
//! See [`OrderIndependentTransparencyPlugin`] for the trade-offs of using OIT.
//!
//! [`OrderIndependentTransparencyPlugin`]: bevy::core_pipeline::oit::OrderIndependentTransparencyPlugin
use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::css::{BLUE, GREEN, RED, YELLOW},
    core_pipeline::{oit::OrderIndependentTransparencySettings, prepass::DepthPrepass},
    prelude::*,
};

fn main() {
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
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Add this component to this camera to render transparent meshes using OIT
        OrderIndependentTransparencySettings::default(),
        RenderLayers::layer(1),
        // Msaa currently doesn't work with OIT
        Msaa::Off,
        // Optional: depth prepass can help OIT filter out fragments occluded by opaque objects
        DepthPrepass,
    ));

    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        RenderLayers::layer(1),
    ));

    // spawn help text
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        RenderLayers::layer(1),
        children![
            TextSpan::new("Press T to toggle OIT\n"),
            TextSpan::new("OIT Enabled"),
            TextSpan::new("\nPress C to cycle test scenes"),
        ],
    ));

    // spawn default scene
    spawn_spheres(&mut commands, &mut meshes, &mut materials);
}

fn toggle_oit(
    mut commands: Commands,
    text: Single<Entity, With<Text>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q: Single<(Entity, Has<OrderIndependentTransparencySettings>), With<Camera3d>>,
    mut text_writer: TextUiWriter,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        let (e, has_oit) = *q;
        *text_writer.text(*text, 2) = if has_oit {
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
    q: Query<Entity, With<Mesh3d>>,
    mut scene_id: Local<usize>,
    asset_server: Res<AssetServer>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        // despawn current scene
        for e in &q {
            commands.entity(e).despawn();
        }
        // increment scene_id
        *scene_id = (*scene_id + 1) % 4;
        // spawn next scene
        match *scene_id {
            0 => spawn_spheres(&mut commands, &mut meshes, &mut materials),
            1 => spawn_quads(&mut commands, &mut meshes, &mut materials),
            2 => spawn_occlusion_test(&mut commands, &mut meshes, &mut materials),
            3 => {
                spawn_auto_instancing_test(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    asset_server,
                );
            }
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

    let alpha = 0.25;

    let render_layers = RenderLayers::layer(1);

    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_a + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_b + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_c + offset),
        render_layers.clone(),
    ));
}

fn spawn_quads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let quad_handle = meshes.add(Rectangle::new(3.0, 3.0).mesh());
    let render_layers = RenderLayers::layer(1);
    let xform = |x, y, z| {
        Transform::from_rotation(Quat::from_rotation_y(0.5))
            .mul_transform(Transform::from_xyz(x, y, z))
    };
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        xform(1.0, -0.1, 0.),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(0.8).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        xform(0.5, 0.2, -0.5),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_green(1.0).with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        xform(0.0, 0.4, -1.),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: YELLOW.with_alpha(0.3).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        xform(-0.5, 0.6, -1.1),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(0.2).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        xform(-0.8, 0.8, -1.2),
        render_layers.clone(),
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
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, 2.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(x, 0., 0.),
        render_layers.clone(),
    ));

    // intersection
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, 1.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0., 0., 0.),
        render_layers.clone(),
    ));

    // back
    let x = 2.5;
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, -2.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(x, 0., 0.),
        render_layers.clone(),
    ));
}

fn spawn_auto_instancing_test(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: Res<AssetServer>,
) {
    let render_layers = RenderLayers::layer(1);

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material_handle = materials.add(StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        base_color_texture: Some(asset_server.load("textures/slice_square.png")),
        ..Default::default()
    });
    let mut bundles = Vec::with_capacity(3 * 3 * 3);

    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                bundles.push((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    Transform::from_xyz(x as f32 * 2.0, y as f32 * 2.0, z as f32 * 2.0),
                    render_layers.clone(),
                ));
            }
        }
    }
    commands.spawn_batch(bundles);
}
