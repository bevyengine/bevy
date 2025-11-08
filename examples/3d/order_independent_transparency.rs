//! A simple 3D scene showing how alpha blending can break and how order independent transparency (OIT) can fix it.
//!
//! See [`ExactOitPlugin`] and [`WeightedBlendedOitPlugin`] for the trade-offs of using OIT.
//!
//! [`ExactOitPlugin`]: bevy::core_pipeline::oit::ExactOitPlugin
//! [`WeightedBlendedOitPlugin`]: bevy::core_pipeline::oit::WeightedBlendedOitPlugin
use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::css::{BLUE, GREEN, RED, YELLOW},
    core_pipeline::oit::{ExactOit, WeightedBlendedOit},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_oit, cycle_scenes).chain())
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
        ExactOit::default(),
        RenderLayers::layer(1),
        // Msaa currently doesn't work with OIT
        Msaa::Off,
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: false,
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
            TextSpan::new("Press T to cycle OIT methods\n"),
            TextSpan::new("OIT method: exact"),
            TextSpan::new("\nPress C to cycle test scenes"),
        ],
    ));

    commands.insert_resource(MaterialAlphaMode(AlphaMode::UnsortedBlend));
    commands.insert_resource(SceneId(0));

    // spawn default scene
    spawn_spheres(
        &mut commands,
        &mut meshes,
        &mut materials,
        MaterialAlphaMode(AlphaMode::UnsortedBlend),
    );
}

fn toggle_oit(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    text: Single<Entity, With<Text>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    cam: Single<(Entity, Has<ExactOit>, Has<WeightedBlendedOit>), With<Camera3d>>,
    q: Query<Entity, With<Mesh3d>>,
    mut material_alpha_mode: ResMut<MaterialAlphaMode>,
    scene_id: Res<SceneId>,
    mut text_writer: TextUiWriter,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        let (e, exact_oit, wb_oit) = *cam;
        *text_writer.text(*text, 2) = {
            if wb_oit {
                commands.entity(e).remove::<ExactOit>();
                commands.entity(e).remove::<WeightedBlendedOit>();
                material_alpha_mode.0 = AlphaMode::Blend;
                "OIT disabled".to_string()
            } else if exact_oit {
                commands.entity(e).remove::<ExactOit>();
                commands.entity(e).insert(WeightedBlendedOit);
                material_alpha_mode.0 = AlphaMode::WeightedBlend;
                "OIT method: weighted blend".to_string()
            } else {
                commands.entity(e).remove::<WeightedBlendedOit>();
                commands.entity(e).insert(ExactOit::default());
                material_alpha_mode.0 = AlphaMode::UnsortedBlend;
                "OIT method: exact".to_string()
            }
        };
        respawn_scene(
            &mut commands,
            &mut meshes,
            &mut materials,
            *material_alpha_mode,
            *scene_id,
            q,
        );
    }
}

#[derive(Resource, Clone, Copy)]
struct MaterialAlphaMode(AlphaMode);

#[derive(Resource, Clone, Copy)]
struct SceneId(u8);

fn cycle_scenes(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<Entity, With<Mesh3d>>,
    mut scene_id: ResMut<SceneId>,
    material_alpha_mode: Res<MaterialAlphaMode>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        // increment scene_id
        scene_id.0 = (scene_id.0 + 1) % 3;
        // spawn next scene
        respawn_scene(
            &mut commands,
            &mut meshes,
            &mut materials,
            *material_alpha_mode,
            *scene_id,
            q,
        );
    }
}

fn respawn_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    material_alpha_mode: MaterialAlphaMode,
    scene_id: SceneId,
    q: Query<Entity, With<Mesh3d>>,
) {
    // despawn current scene
    for e in &q {
        commands.entity(e).despawn();
    }
    match scene_id.0 {
        0 => spawn_spheres(commands, meshes, materials, material_alpha_mode),
        1 => spawn_quads(commands, meshes, materials, material_alpha_mode),
        2 => spawn_occlusion_test(commands, meshes, materials, material_alpha_mode),
        _ => unreachable!(),
    }
}

/// Spawns 3 overlapping spheres
/// Technically, when using `alpha_to_coverage` with MSAA this particular example wouldn't break,
/// but it breaks when disabling MSAA and is enough to show the difference between OIT enabled vs disabled.
fn spawn_spheres(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    material_alpha_mode: MaterialAlphaMode,
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
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_translation(pos_a + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_alpha(alpha).into(),
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_translation(pos_b + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(alpha).into(),
            alpha_mode: material_alpha_mode.0,
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
    material_alpha_mode: MaterialAlphaMode,
) {
    let quad_handle = meshes.add(Rectangle::new(3.0, 3.0).mesh());
    let render_layers = RenderLayers::layer(1);

    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_xyz(1.0, 0., 0.).with_rotation(Quat::from_rotation_y(0.5)),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(1.0).into(),
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_xyz(0.5, 0., -0.5).with_rotation(Quat::from_rotation_y(0.5)),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_green(1.0).with_alpha(0.5).into(),
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_xyz(0.0, 0., -1.)
            .with_scale(Vec3::new(1.0, 1.2, 1.0))
            .with_rotation(Quat::from_rotation_y(0.5)),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: YELLOW.with_alpha(0.5).into(),
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_xyz(-0.5, 0., -1.).with_rotation(Quat::from_rotation_y(0.5)),
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
    material_alpha_mode: MaterialAlphaMode,
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
            alpha_mode: material_alpha_mode.0,
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
            alpha_mode: material_alpha_mode.0,
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
            alpha_mode: material_alpha_mode.0,
            ..default()
        })),
        Transform::from_xyz(x, 0., 0.),
        render_layers.clone(),
    ));
}
