//! A simple UI health bar which follows an object around in 3D space.
//! Using UI nodes is just one way to do this. Alternatively, you can use
//! a mesh facing the camera to set up your health bar.

use bevy::color::palettes::css::{GREEN, RED};
use bevy::color::ColorCurve;
use bevy::prelude::*;

const BAR_HEIGHT: f32 = 25.0;
const BAR_WIDTH: f32 = 150.0;
const HALF_BAR_HEIGHT: f32 = BAR_HEIGHT / 2.0;
const HALF_BAR_WIDTH: f32 = BAR_WIDTH / 2.0;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct HealthBar {
    /// The target entity that the health bar should follow
    target: Entity,
    health_text_entity: Entity,
    color_curve: ColorCurve<LinearRgba>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_health, update_health_bar, move_cube))
        .run();
}

/// set up a 3D scene where the cube will have a health bar
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.5, 2.5, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // cube with a health component
    let cube_id = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 0.5, 0.0),
            Health(42.0),
        ))
        .id();
    // Root component for the health bar, this one will be moved to follow the cube
    let health_bar_root = commands
        .spawn((
            Node {
                width: Val::Px(BAR_WIDTH),
                height: Val::Px(BAR_HEIGHT),
                padding: UiRect::all(Val::Px(4.)),
                display: Display::Flex,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();

    let health_text = commands
        .spawn((
            Node {
                min_width: Val::Px(30.0),
                margin: UiRect::left(Val::Px(2.0)),
                ..default()
            },
            Text::new("42"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
        ))
        .id();

    // Define the control points for the color curve.
    // For more information, please see the cubic curve example.
    let colors = [
        LinearRgba::RED,
        LinearRgba::RED,
        LinearRgba::rgb(1., 1., 0.), // Yellow
        LinearRgba::GREEN,
    ];

    let health_bar_nodes = commands
        .spawn((
            Node {
                align_items: AlignItems::Stretch,
                width: Val::Percent(100.),
                ..default()
            },
            HealthBar {
                target: cube_id,
                health_text_entity: health_text,
                color_curve: ColorCurve::new(colors).unwrap(),
            },
            BackgroundColor(Color::from(GREEN)),
        ))
        .id();

    commands
        .entity(health_bar_root)
        .add_children(&[health_text, health_bar_nodes]);
}

// Some placeholder system to affect the health in this example.
fn update_health(time: Res<Time>, mut health_query: Query<&mut Health>) {
    for mut health in health_query.iter_mut() {
        health.0 = (time.elapsed().as_secs_f32().sin() + 1.0) * 50.0;
    }
}

fn update_health_bar(
    mut health_bar_query: Query<(&mut Node, &HealthBar, &ChildOf, &mut BackgroundColor)>,
    mut health_bar_root_query: Query<&mut Node, Without<HealthBar>>,
    mut health_bar_text_query: Query<&mut Text, Without<HealthBar>>,
    target_query: Query<(&GlobalTransform, &Health)>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
) {
    let camera = camera_query.0;
    let cam_transform = camera_query.1;

    for (mut health_bar_node, health_bar_component, child_of, mut bg_color) in
        health_bar_query.iter_mut()
    {
        let mut root = health_bar_root_query.get_mut(child_of.0).unwrap();
        let mut health_text = health_bar_text_query
            .get_mut(health_bar_component.health_text_entity)
            .unwrap();
        let (target, target_health) = target_query.get(health_bar_component.target).unwrap();

        let target_world_position = target.translation();
        let target_viewport_position = camera
            .world_to_viewport(cam_transform, target_world_position)
            .unwrap();

        root.left = Val::Px(target_viewport_position.x - HALF_BAR_WIDTH);
        root.top = Val::Px(target_viewport_position.y - HALF_BAR_HEIGHT);

        let hp = target_health.0;

        // todo: A width beyond roughly 90% doesn't seem to make a difference
        health_bar_node.width = Val::Percent(hp);
        health_text.0 = format!("{:.0}", hp); // Only show rounded numbers

        let color_curve = &health_bar_component.color_curve;
        let t = hp * 4.0 / (100.0); // 4 is the number of colors, 100 is the max health
        bg_color.0 = color_curve.sample_clamped(t).into();
    }
}

// Some placeholder movement so that we can see that the
// health bar is correctly following the cube around
fn move_cube(time: Res<Time>, mut movables: Query<&mut Transform, With<Health>>) {
    for mut transform in movables.iter_mut() {
        transform.translation.x = time.elapsed_secs().sin() * 2.0;
        transform.translation.z = time.elapsed_secs().cos() * 2.0;
    }
}
