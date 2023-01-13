use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_cube)
        .run();
}

fn move_cube(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut cube_query: Query<&mut Transform, With<Cube>>,
    windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
) {
    let (camera, camera_transform) = camera_query.single();
    let mut transform = cube_query.single_mut();

    if !input.pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_position) = windows.primary().cursor_position() else { return; };

    let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) else { return; };

    let Some(distance) = ray.intersect_plane(Vec3::ZERO, Vec3::Y) else { return; };

    let point = ray.get_point(distance);
    transform.translation = point + Vec3::Y * 0.5;
}

#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 50.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // cube
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(Cube);

    // light
    const HALF_SIZE: f32 = 15.0;
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            ..default()
        },
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // UI
    commands.spawn(TextBundle::from_section(
        "Press the left mouse button to reposition the box.",
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 32.,
            ..default()
        },
    ));
}
