// based on: https://antongerdelan.net/opengl/raycasting.html

use bevy::{
    input::{
        keyboard::ElementState,
        mouse::MouseButtonInput,
    },
    prelude::*,
    window::WindowId,
};

use bevy::render::camera::CameraProjection;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(ray_cast_mouse.system())
        .run();
}

#[derive(Default)]
struct MouseState {
    mouse_button_event_reader: EventReader<MouseButtonInput>,
    cursor_moved_event_reader: EventReader<CursorMoved>,
    cursor_pos: Vec2,
    cursor_window_id: Option<WindowId>,
}

struct GreenBallTag;

struct MainCameraEntity(pub Entity);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    commands.spawn(PbrComponents {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        material: materials.add(StandardMaterial {
            albedo: Color::WHITE,
            ..Default::default()
        }),
        ..Default::default()
    });

    commands.spawn(LightComponents {
        translation: Translation(Vec3::new(10.0, 10.0, 10.0)),
        ..Default::default()
    });

    // Example Ball
    commands
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 3,
            })),
            material: materials.add(StandardMaterial {
                albedo: Color::GREEN,
                ..Default::default()
            }),
            transform: Transform::new(Mat4::identity()),
            ..Default::default()
        })
        .with(GreenBallTag);

    // Camera
    let main_camera_entity = Entity::new();
    commands
        .spawn_as_entity(
            main_camera_entity,
            Camera3dComponents {
                transform: Transform::new_sync_disabled(Mat4::face_toward(
                    Vec3::new(10.0, 10.0, 10.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                )),
                ..Default::default()
            },
        )
        .insert_resource(MainCameraEntity(main_camera_entity));

    commands.insert_resource(MouseState::default());
}

fn ray_cast_mouse(
    mut state: ResMut<MouseState>,
    mouse_button_input_events: Res<Events<MouseButtonInput>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    main_camera_entity: Res<MainCameraEntity>,
    mut placeable: Query<With<GreenBallTag, (&mut Translation)>>,
    mut cameras: Query<(
        &Transform,
        &bevy::render::camera::Camera,
        &bevy::render::camera::PerspectiveProjection,
    )>,
) {
    for event in state.cursor_moved_event_reader.iter(&cursor_moved_events) {
        state.cursor_pos = event.position;
        state.cursor_window_id = Some(event.id); // get the window id here, as it is only available in the moved mouse event
    }

    for event in state
        .mouse_button_event_reader
        .iter(&mouse_button_input_events)
    {
        if let Some(window_id) = state.cursor_window_id {
            if event.button == MouseButton::Left && event.state == ElementState::Pressed {
                let window = windows.get(window_id).unwrap();

                let mut main_camera_ent = cameras.entity(main_camera_entity.0).unwrap();
                let main_camera = main_camera_ent.get().unwrap();
                let mouse_ray = cursor_pos_to_ray(
                    &state.cursor_pos,
                    &window,
                    &main_camera.0.value,
                    &main_camera.2.get_projection_matrix(),
                );

                let plane = Plane::new(Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
                if let Some(hit_point) = plane.intersect_ray(&mouse_ray) {
                    for mut translation in &mut placeable.iter() {
                        translation.0 = hit_point;
                    }
                }
            }
        }
    }
}

fn cursor_pos_to_ray(
    cursor_viewport: &Vec2,
    window: &Window,
    camera_transform: &Mat4,
    camera_perspective: &Mat4,
) -> Ray {
    // calculate the cursor pos in NDC space [(-1,-1), (1,1)]
    let cursor_ndc = Vec4::from((
        (cursor_viewport.x() / window.width as f32) * 2.0 - 1.0,
        (cursor_viewport.y() / window.height as f32) * 2.0 - 1.0,
        -1.0, // let the cursor be on the far clipping plane
        1.0,
    ));

    let object_to_world = camera_transform;
    let object_to_ndc = camera_perspective;

    // transform the cursor position into object/camera space
    // this also turns the cursor into a vector that's pointing from the camera center onto the far plane
    let mut ray_camera = object_to_ndc.inverse().mul_vec4(cursor_ndc);
    ray_camera.set_z(-1.0);
    ray_camera.set_w(0.0); // treat the vector as a direction (0 = Direction, 1 = Position)

    // transform the cursor into world space
    let ray_world = object_to_world.mul_vec4(ray_camera);
    let ray_world = Vec3::from(ray_world.truncate());

    let camera_pos = Vec3::from(camera_transform.w_axis().truncate());
    Ray {
        origin: camera_pos,
        direction: (ray_world).normalize(),
    }
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction }
    }
}

#[derive(Debug)]
pub struct Plane {
    pub origin: Vec3,
    pub normal: Vec3,
}

impl Plane {
    pub fn new(origin: Vec3, direction: Vec3) -> Plane {
        Plane {
            origin,
            normal: direction,
        }
    }

    pub fn intersect_ray(&self, ray: &Ray) -> Option<Vec3> {
        intersect_ray_plane(ray, self)
    }
}

// Intersection Methods
//==================================================================================================
fn intersect_ray_plane(ray: &Ray, plane: &Plane) -> Option<Vec3> {
    let denominator = plane.normal.dot(ray.direction);
    if denominator.abs() > f32::EPSILON {
        let t = (plane.origin - ray.origin).dot(plane.normal) / denominator;
        if t >= f32::EPSILON {
            return Some(ray.origin + t * ray.direction);
        }
    }
    return None;
}
