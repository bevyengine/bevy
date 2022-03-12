use bevy::{
    asset::{AssetServerSettings, LoadState},
    input::mouse::MouseMotion,
    prelude::*,
    render::{
        camera::{CameraPlugin, CameraProjection},
        primitives::{Aabb, Frustum},
    },
};

fn main() {
    println!(
        "
Controls:
    WSAD   - forward/back/strafe left/right
    LShift - 'run'
    E      - up
    Q      - down
    L      - animate light direction
    U      - toggle shadows
    5/6    - decrease/increase shadow projection width
    7/8    - decrease/increase shadow projection height
    9/0    - decrease/increase shadow projection near/far
"
    );
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(AssetServerSettings {
            asset_folder: std::env::var("CARGO_MANIFEST_DIR").unwrap(),
            watch_for_changes: true,
        })
        .insert_resource(WindowDescriptor {
            title: "bevy scene viewer".to_string(),
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::PreUpdate, scene_load_check)
        .add_system_to_stage(CoreStage::PreUpdate, camera_spawn_check)
        .add_system(camera_controller_check.label("camera_controller_check"))
        .add_system(update_lights)
        .add_system(camera_controller.after("camera_controller_check"))
        .run();
}

struct SceneHandle {
    handle: Option<Handle<Scene>>,
    has_camera: bool,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/models/FlightHelmet/FlightHelmet.gltf#Scene0".to_string());
    commands.insert_resource(SceneHandle {
        handle: Some(asset_server.load(&scene_path)),
        has_camera: false,
    });
}

fn scene_load_check(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Assets<Scene>>,
    mut scene_handle: ResMut<SceneHandle>,
) {
    if scene_handle.handle.is_some()
        && asset_server.get_load_state(scene_handle.handle.as_ref().unwrap()) == LoadState::Loaded
    {
        let mut to_remove = Vec::new();

        let scene = scenes
            .get_mut(scene_handle.handle.as_ref().unwrap())
            .unwrap();
        let mut query = scene.world.query::<(Entity, &Camera)>();
        for (entity, camera) in query.iter(&scene.world) {
            match camera.name.as_deref() {
                Some(CameraPlugin::CAMERA_3D) if !scene_handle.has_camera => {
                    scene_handle.has_camera = true;
                    println!("Model has a 3D camera");
                }
                Some(CameraPlugin::CAMERA_2D) if !scene_handle.has_camera => {
                    scene_handle.has_camera = true;
                    println!("Model has a 2D camera");
                }
                _ => {
                    to_remove.push(entity);
                }
            }
        }

        for entity in to_remove.drain(..) {
            scene.world.entity_mut(entity).despawn_recursive();
        }

        commands.spawn_scene(scene_handle.handle.take().unwrap());
        println!("Spawning scene");
    }
}

fn camera_spawn_check(
    mut commands: Commands,
    mut scene_handle: ResMut<SceneHandle>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Handle<Mesh>>>,
) {
    // scene_handle.handle.is_none() indicates that the scene has been spawned
    // If the scene did not contain a camera, find an approximate bounding box of the scene from
    // its meshes and spawn a camera that fits it in view
    if scene_handle.handle.is_none() && !scene_handle.has_camera {
        if meshes.iter().any(|(_, maybe_aabb)| maybe_aabb.is_none()) {
            return;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for (transform, maybe_aabb) in meshes.iter() {
            let aabb = maybe_aabb.unwrap();
            // This isn't fully correct for finding the min/max but should be good enough
            min = min.min(transform.mul_vec3(aabb.min()));
            max = max.max(transform.mul_vec3(aabb.max()));
        }

        let size = (max - min).length();
        let center = 0.5 * (max + min);

        let transform = Transform::from_translation(center + size * Vec3::new(1.0, 0.5, 1.0))
            .looking_at(center, Vec3::Y);
        let view = transform.compute_matrix();
        let mut perspective_projection = PerspectiveProjection::default();
        perspective_projection.far = perspective_projection.far.max(size * 10.0);
        let view_projection = view.inverse() * perspective_projection.get_projection_matrix();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &transform.translation,
            &transform.back(),
            perspective_projection.far(),
        );

        println!("Spawning a 3D perspective camera");
        commands.spawn_bundle(PerspectiveCameraBundle {
            camera: Camera {
                name: Some(CameraPlugin::CAMERA_3D.to_string()),
                near: perspective_projection.near,
                far: perspective_projection.far,
                ..default()
            },
            perspective_projection,
            frustum,
            transform,
            ..PerspectiveCameraBundle::new_3d()
        });

        scene_handle.has_camera = true;
    }
}

// TODO: Register all types in CameraController so that it can be registered
fn camera_controller_check(
    mut commands: Commands,
    camera: Query<Entity, (With<Camera>, Without<CameraController>)>,
) {
    if let Ok(entity) = camera.get_single() {
        commands.entity(entity).insert(CameraController::default());
    }
}

const SCALE_STEP: f32 = 0.1;

fn update_lights(
    key_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut DirectionalLight)>,
    mut animate_directional_light: Local<bool>,
) {
    let mut projection_adjustment = Vec3::ONE;
    if key_input.just_pressed(KeyCode::Key5) {
        projection_adjustment.x -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key6) {
        projection_adjustment.x += SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key7) {
        projection_adjustment.y -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key8) {
        projection_adjustment.y += SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key9) {
        projection_adjustment.z -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key0) {
        projection_adjustment.z += SCALE_STEP;
    }
    for (_, mut light) in query.iter_mut() {
        light.shadow_projection.left *= projection_adjustment.x;
        light.shadow_projection.right *= projection_adjustment.x;
        light.shadow_projection.bottom *= projection_adjustment.y;
        light.shadow_projection.top *= projection_adjustment.y;
        light.shadow_projection.near *= projection_adjustment.z;
        light.shadow_projection.far *= projection_adjustment.z;
        if key_input.just_pressed(KeyCode::U) {
            light.shadows_enabled = !light.shadows_enabled;
        }
    }

    if key_input.just_pressed(KeyCode::L) {
        *animate_directional_light = !*animate_directional_light;
    }
    if *animate_directional_light {
        for (mut transform, _) in query.iter_mut() {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.seconds_since_startup() as f32 * std::f32::consts::TAU / 10.0,
                -std::f32::consts::FRAC_PI_4,
            );
        }
    }
}

#[derive(Component)]
struct CameraController {
    pub enabled: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.5,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            walk_speed: 5.0,
            run_speed: 15.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    key_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    // Handle mouse input
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.iter() {
        mouse_delta += mouse_event.delta;
    }

    for (mut transform, mut options) in query.iter_mut() {
        if !options.enabled {
            continue;
        }

        // Handle key input
        let mut axis_input = Vec3::ZERO;
        if key_input.pressed(options.key_forward) {
            axis_input.z += 1.0;
        }
        if key_input.pressed(options.key_back) {
            axis_input.z -= 1.0;
        }
        if key_input.pressed(options.key_right) {
            axis_input.x += 1.0;
        }
        if key_input.pressed(options.key_left) {
            axis_input.x -= 1.0;
        }
        if key_input.pressed(options.key_up) {
            axis_input.y += 1.0;
        }
        if key_input.pressed(options.key_down) {
            axis_input.y -= 1.0;
        }

        // Apply movement update
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(options.key_run) {
                options.run_speed
            } else {
                options.walk_speed
            };
            options.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = options.friction.clamp(0.0, 1.0);
            options.velocity *= 1.0 - friction;
            if options.velocity.length_squared() < 1e-6 {
                options.velocity = Vec3::ZERO;
            }
        }
        let forward = transform.forward();
        let right = transform.right();
        transform.translation += options.velocity.x * dt * right
            + options.velocity.y * dt * Vec3::Y
            + options.velocity.z * dt * forward;

        if mouse_delta != Vec2::ZERO {
            // Apply look update
            let (pitch, yaw) = (
                (options.pitch - mouse_delta.y * 0.5 * options.sensitivity * dt).clamp(
                    -0.99 * std::f32::consts::FRAC_PI_2,
                    0.99 * std::f32::consts::FRAC_PI_2,
                ),
                options.yaw - mouse_delta.x * options.sensitivity * dt,
            );
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
            options.pitch = pitch;
            options.yaw = yaw;
        }
    }
}
