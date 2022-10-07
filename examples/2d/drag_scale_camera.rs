//! How to change the transform and scale of the camera. Affect camera via dragging and scrolling

use bevy::{input::mouse::MouseWheel, prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(camera_scale)
        .add_system(camer_drag)
        .insert_resource(ClearColor(Color::RED))
        .run();
}

fn camer_drag(
    mouse_button_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Transform, &mut OrthographicProjection)>,
    windows: Res<Windows>,
    mut prev_position: Local<Vec2>,
) {
    if let Some(cursor_position) = windows.primary().cursor_position() {
        if mouse_button_input.pressed(MouseButton::Left) {
            for (mut transform, cam) in query.iter_mut() {
                // Calculate the delta in mouse position when it has been held down.
                let delta: Vec2 = (*prev_position - cursor_position) * cam.scale;

                //Applying the motion delta to the camera's transform
                *transform = Transform::from_translation(transform.mul_vec3(delta.extend(0.0)));
            }
        }
        *prev_position = cursor_position;
    }
}
fn camera_scale(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query: Query<&mut OrthographicProjection, With<Camera>>,
) {
    // Query for mouse scroll events, and will apply delta scroll to camera's scale
    for event in mouse_wheel_events.iter() {
        for mut proj in query.iter_mut() {
            let mut log_scale = proj.scale.ln();
            log_scale += event.y * 0.5;
            proj.scale = log_scale.exp();
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let square_size = 10;

    // Quick loop to generate array of circles as a reference point for background
    for x in -square_size..square_size {
        for y in -square_size..square_size {
            commands.spawn(MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(10.).into()).into(),
                material: materials.add(ColorMaterial::from(Color::BLUE)),
                transform: Transform::from_translation(Vec3::new(
                    (x as f32) * 100.0,
                    (y as f32) * 100.0,
                    0.,
                )),
                ..default()
            });
        }
    }
}
