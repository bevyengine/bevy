//! A simple UI health bar which follows an object around in 3D space.
//! Using UI nodes is just one way to do this. Alternatively, you can use
//! a mesh facing the camera to set up your health bar.

use bevy::color::palettes::basic::{BLACK, GREEN};
use bevy::color::ColorCurve;
use bevy::math::ops::{cos, sin};
use bevy::prelude::*;
use bevy::transform::plugins::TransformSystems;
use bevy::ui::UiSystems;

const BAR_HEIGHT: f32 = 25.0;
const BAR_WIDTH: f32 = 160.0;
const HALF_BAR_HEIGHT: f32 = BAR_HEIGHT / 2.0;
const HALF_BAR_WIDTH: f32 = BAR_WIDTH / 2.0;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct HealthBar {
    /// The target entity that the health bar should follow
    target: Entity,
    health_text_entity: Entity,
    root_ui_entity: Entity,
    color_curve: ColorCurve<LinearRgba>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_cube, update_health).chain())
        .add_systems(
            // Bevy's UI Layout happens before transform propagation,
            // so we will have to run update_health_bar before both
            // and do the transform propagation manually.
            PostUpdate,
            update_health_bar
                .before(TransformSystems::Propagate)
                .before(UiSystems::Layout),
        )
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
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .id();

    let health_text = commands
        .spawn((
            Node::default(),
            Text::new("42"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextShadow {
                offset: Vec2::splat(2.0),
                color: BLACK.into(),
            },
        ))
        .id();

    let health_bar_background = commands
        .spawn((
            Node {
                width: Val::Px(BAR_WIDTH),
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(BLACK.into()),
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

    let health_bar_node = commands
        .spawn((
            Node {
                align_items: AlignItems::Stretch,
                width: Val::Percent(100.),
                height: Val::Px(BAR_HEIGHT),
                border: UiRect::all(Val::Px(4.)),
                ..default()
            },
            HealthBar {
                target: cube_id,
                health_text_entity: health_text,
                root_ui_entity: health_bar_root,
                color_curve: ColorCurve::new(colors).unwrap(),
            },
            BackgroundColor(Color::from(GREEN)),
        ))
        .id();

    commands
        .entity(health_bar_root)
        .add_children(&[health_text, health_bar_background]);

    commands
        .entity(health_bar_background)
        .add_child(health_bar_node);
}

// Some placeholder system to affect the health in this example.
fn update_health(time: Res<Time>, mut health_query: Query<&mut Health>) {
    for mut health in health_query.iter_mut() {
        health.0 = (sin(time.elapsed().as_secs_f32()) + 1.0) * 50.0;
    }
}

fn update_health_bar(
    mut health_bar_query: Query<(&mut Node, &HealthBar, &mut BackgroundColor)>,
    mut health_bar_root_query: Query<&mut Node, Without<HealthBar>>,
    mut health_bar_text_query: Query<&mut Text, Without<HealthBar>>,
    target_query: Query<(Entity, &Health)>,
    camera_query: Single<(Entity, &Camera)>,
    transform_helper: TransformHelper,
) {
    let camera_entity = camera_query.0;
    let camera = camera_query.1;

    // Since the global transform is not propagated at this point (see system ordering comment),
    // we will calculate the global transform manually:
    let Ok(camera_transform) = transform_helper.compute_global_transform(camera_entity) else {
        warn!("Failed computing global transform for camera Entity");
        return;
    };

    for (mut health_bar_node, health_bar_component, mut bg_color) in health_bar_query.iter_mut() {
        let root_entity = health_bar_component.root_ui_entity;
        let mut root_node = health_bar_root_query.get_mut(root_entity).unwrap();
        let mut health_text = health_bar_text_query
            .get_mut(health_bar_component.health_text_entity)
            .unwrap();
        let (target, target_health) = target_query.get(health_bar_component.target).unwrap();

        let Ok(target_world_transform) = transform_helper.compute_global_transform(target) else {
            warn!("Failed computing global transform for target Entity");
            return;
        };
        let target_world_position = target_world_transform.translation();

        let target_viewport_position = camera
            .world_to_viewport(&camera_transform, target_world_position)
            .unwrap();

        root_node.left = Val::Px(target_viewport_position.x - HALF_BAR_WIDTH);
        root_node.top = Val::Px(target_viewport_position.y - HALF_BAR_HEIGHT);

        let hp = target_health.0;

        health_bar_node.width = Val::Percent(hp);
        health_text.0 = format!("Health: {:.0} / 100", hp); // Only show rounded numbers

        let color_curve = &health_bar_component.color_curve;
        let t = hp * 4.0 / (100.0); // 4 is the number of colors, 100 is the max health
        bg_color.0 = color_curve.sample_clamped(t).into();
    }
}

// Some placeholder movement so that we can see that the
// health bar is correctly following the cube around
fn move_cube(time: Res<Time>, mut movables: Query<&mut Transform, With<Health>>) {
    for mut transform in movables.iter_mut() {
        transform.translation.x = sin(time.elapsed_secs()) * 2.0;
        transform.translation.z = cos(time.elapsed_secs()) * 2.0;
    }
}
