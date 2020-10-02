use bevy::{
    input::{keyboard::ElementState, mouse::MouseButtonInput},
    physics::d3::prelude::*,
    prelude::*,
    render::camera::Camera,
};

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(raycast.system())
        .add_system(move_camera.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
            material: materials.add(Color::rgb(0.1, 0.2, 0.1).into()),
            ..Default::default()
        })
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: 0.5,
            })),
            material: materials.add(Color::rgb(0.1, 0.4, 0.8).into()),
            transform: Transform::from_translation(Vec3::new(1.5, 1.5, 0.0)),
            ..Default::default()
        })
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        })
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .insert_resource(MouseState::default());
}

#[derive(Default)]
struct MouseState {
    mouse_button_event_reader: EventReader<MouseButtonInput>,
    cursor_moved_event_reader: EventReader<CursorMoved>,
    cursor_position: Vec2,
}

fn raycast(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mouse_state: ResMut<MouseState>,
    mouse_button_input_events: Res<Events<MouseButtonInput>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    mut cameras: Query<(&Camera, &GlobalTransform)>,
) {
    for event in mouse_state
        .cursor_moved_event_reader
        .iter(&cursor_moved_events)
    {
        mouse_state.cursor_position = event.position;
    }

    for event in mouse_state
        .mouse_button_event_reader
        .iter(&mouse_button_input_events)
    {
        if event.button == MouseButton::Left && event.state == ElementState::Pressed {
            for (camera, global_transform) in &mut cameras.iter() {
                let window = windows.get(camera.window).unwrap();

                let ray = Ray::from_mouse_position(
                    &mouse_state.cursor_position,
                    window,
                    camera,
                    global_transform,
                );

                let plane_hit = Plane::new(Vec3::zero(), Vec3::unit_y()).intersect_ray(&ray);
                let sphere_hit = Sphere {
                    center: Vec3::new(1.5, 1.5, 0.0),
                    radius: 0.5,
                }
                .intersect_ray(&ray);

                let hit = if let Some(plane_hit) = plane_hit {
                    if let Some(sphere_hit) = sphere_hit {
                        if plane_hit.t() < sphere_hit.t() {
                            Some(plane_hit)
                        } else {
                            Some(sphere_hit)
                        }
                    } else {
                        Some(plane_hit)
                    }
                } else if let Some(sphere_hit) = sphere_hit {
                    Some(sphere_hit)
                } else {
                    None
                };

                if let Some(hit) = hit {
                    commands.spawn(PbrComponents {
                        mesh: meshes.add(Mesh::from(shape::Icosphere {
                            subdivisions: 3,
                            radius: 0.05,
                        })),
                        material: materials.add(Color::RED.into()),
                        transform: Transform::from_translation(*hit.point()),
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn move_camera(keyboard_input: Res<Input<KeyCode>>, mut cameras: Query<(&mut Transform, &Camera)>) {
    let speed = 0.1;

    let translation = Vec3::unit_x()
        * speed
        * if keyboard_input.pressed(KeyCode::A) {
            -1.0
        } else if keyboard_input.pressed(KeyCode::D) {
            1.0
        } else {
            0.0
        };

    for (mut camera_transform, _) in &mut cameras.iter() {
        let rotation = camera_transform.rotation();
        camera_transform.translate(rotation * translation);
        let position = camera_transform.translation();
        camera_transform.set_rotation(Quat::from_rotation_mat4(&Mat4::face_toward(
            position,
            Vec3::zero(),
            Vec3::unit_y(),
        )));
    }
}
