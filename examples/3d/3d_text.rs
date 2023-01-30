//! This example shows how to render text in a 3d space

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate_billboard)
        .add_system(rotate_camera)
        .add_system(update_frame_counter)
        .run();
}

/// A text that always faces the camera
#[derive(Component)]
struct Billboard {
    /// determines if the billboard looks up at the camera, or only looks in the x/y direction
    rotate_z: bool,
}
/// A text that shows the frame counter
#[derive(Component)]
struct Framecounter;

/// sets up a scene with textured entities
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let font = assets.load("fonts/FiraMono-Medium.ttf");
    let style = TextStyle {
        color: Color::BLACK,
        font,
        font_size: 32.,
    };
    // A billboard that always looks directly at the camera
    commands.spawn((
        Text3dBundle {
            text: Text::from_section("Looking straight at you", style.clone()),
            ..default()
        },
        Billboard { rotate_z: true },
    ));

    // A billboard that always looks in the direction of the camera, but does not change the z-angle
    // this is useful for e.g. nametags above players
    commands.spawn((
        Text3dBundle {
            text: Text::from_section("Looking in your general direction", style.clone()),
            ..default()
        },
        Billboard { rotate_z: false },
    ));

    // A generic text. We use the `Framecounter` tag to update this value every frame
    commands.spawn((
        Text3dBundle {
            text: Text::from_section("Frame counter goes here", style.clone()),
            ..default()
        },
        Framecounter,
    ));

    // Add a ground plane so we can see what we're rotating around
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(10., 10.)))),
        material: materials.add(StandardMaterial {
            base_color: Color::rgba(0.0, 1.0, 0.0, 0.0),
            ..default()
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(15.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Z),
        ..default()
    });
    // light
    commands.insert_resource(AmbientLight {
        brightness: 1.0,
        color: Color::WHITE,
    });
    // action
}

fn rotate_billboard(
    mut billboards: Query<(&mut Transform, &Billboard), Without<Camera>>,
    camera: Query<&Transform, With<Camera>>,
) {
    let camera = camera.single();
    for (mut transform, billboard) in billboards.iter_mut() {
        let mut look_at = camera.translation;
        if !billboard.rotate_z {
            look_at.z = transform.translation.z;
        }
        transform.look_at(look_at, Vec3::Z);
    }
}
fn rotate_camera(mut camera: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    const SPEED: f32 = 0.5;
    let mut transform = camera.single_mut();
    transform.rotate_around(
        Vec3::ZERO,
        Quat::from_rotation_z(SPEED * time.delta_seconds()),
    );
}
fn update_frame_counter(mut text: Query<&mut Text, With<Framecounter>>, mut counter: Local<u32>) {
    *counter += 1;
    for mut text in text.iter_mut() {
        text.sections[0].value = counter.to_string();
    }
}
