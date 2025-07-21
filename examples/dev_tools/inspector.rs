//! Showcase how to use the in-game inspector for debugging entities and components.

use bevy::{
    prelude::*,
    dev_tools::inspector::live_editor::LiveEditorPlugin,

};

// Example component for demonstration
#[derive(Component)]
struct Player {
    speed: f32,
    health: i32,
}

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
        .add_plugins(LiveEditorPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, remove.run_if(input_just_pressed(KeyCode::Space)))
        .add_systems(Update, move_cube)
        // New types must be registered in order to be usable with reflection.
        .register_type::<Cube>()
        .register_type::<TestResource>()
        .run();
}

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

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Cube(1.0),
    ));

    // test resource
    commands.insert_resource(TestResource {
        foo: Vec2::new(1.0, -1.0),
        bar: false,
    });

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
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// An arbitrary resource that can be inspected and manipulated with remote methods.
#[derive(Resource, Reflect, Serialize, Deserialize)]
#[reflect(Resource, Serialize, Deserialize)]
pub struct TestResource {
    /// An arbitrary field of the test resource.
    pub foo: Vec2,

    /// Another arbitrary field.
    pub bar: bool,
}

fn move_cube(mut query: Query<&mut Transform, With<Cube>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.translation.y = -cos(time.elapsed_secs()) + 1.5;
    }
}

fn remove(mut commands: Commands, cube_entity: Single<Entity, With<Cube>>) {
    commands.entity(*cube_entity).remove::<Cube>();
}

#[derive(Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct Cube(f32);
