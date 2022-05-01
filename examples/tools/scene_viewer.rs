use bevy::{
    asset::{AssetServerSettings, LoadState},
    gltf::Gltf,
    input::mouse::MouseMotion,
    math::Vec3A,
    prelude::*,
    render::{
        camera::{ActiveCamera, Camera3d, CameraProjection},
        primitives::{Aabb, Frustum, Sphere},
    },
    scene::InstanceId,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
struct CameraControllerCheckSystem;

fn main() {
    println!(
        "
Controls:
    MOUSE  - Move camera orientation
    LClick - Enable mouse movement
    WSAD   - forward/back/strafe left/right
    LShift - 'run'
    E      - up
    Q      - down
    L      - animate light direction
    U      - toggle shadows
    C      - cycle through cameras
    5/6    - decrease/increase shadow projection width
    7/8    - decrease/increase shadow projection height
    9/0    - decrease/increase shadow projection near/far

    Space  - Play/Pause animation
    Enter  - Cycle through animations
"
    );
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(AssetServerSettings {
            asset_folder: std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
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
        .add_system(update_lights)
        .add_system(camera_controller)
        .add_system(start_animation)
        .add_system(keyboard_animation_control)
        .add_system(keyboard_cameras_control)
        .run();
}

struct SceneHandle {
    handle: Handle<Scene>,
    animations: Vec<Handle<AnimationClip>>,
    instance_id: Option<InstanceId>,
    is_loaded: bool,
    has_camera: bool,
    has_light: bool,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/models/FlightHelmet/FlightHelmet.gltf".to_string());
    info!("Loading {}", scene_path);
    commands.insert_resource(SceneHandle {
        handle: asset_server.load(&scene_path),
        animations: Vec::new(),
        instance_id: None,
        is_loaded: false,
        has_camera: false,
        has_light: false,
    });
}

fn scene_load_check(
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Assets<Scene>>,
    gltf_assets: ResMut<Assets<Gltf>>,
    mut scene_handle: ResMut<SceneHandle>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    match scene_handle.instance_id {
        None => {
            if asset_server.get_load_state(&scene_handle.handle) == LoadState::Loaded {
                let gltf = gltf_assets.get(&scene_handle.handle).unwrap();
                let gltf_scene_handle = gltf.scenes.first().expect("glTF file contains no scenes!");
                let scene = scenes.get_mut(gltf_scene_handle).unwrap();

                let mut query = scene
                    .world
                    .query::<(Option<&DirectionalLight>, Option<&PointLight>)>();
                scene_handle.has_light =
                    query
                        .iter(&scene.world)
                        .any(|(maybe_directional_light, maybe_point_light)| {
                            maybe_directional_light.is_some() || maybe_point_light.is_some()
                        });

                scene_handle.instance_id =
                    Some(scene_spawner.spawn(gltf_scene_handle.clone_weak()));

                scene_handle.animations = gltf.animations.clone();
                if !scene_handle.animations.is_empty() {
                    info!(
                        "Found {} animation{}",
                        scene_handle.animations.len(),
                        if scene_handle.animations.len() == 1 {
                            ""
                        } else {
                            "s"
                        }
                    );
                }

                info!("Spawning scene...");
            }
        }
        Some(instance_id) if !scene_handle.is_loaded => {
            if scene_spawner.instance_is_ready(instance_id) {
                info!("...done!");
                scene_handle.is_loaded = true;
            }
        }
        Some(_) => {}
    }
}

fn start_animation(
    mut player: Query<&mut AnimationPlayer>,
    mut done: Local<bool>,
    scene_handle: Res<SceneHandle>,
) {
    if !*done {
        if let Ok(mut player) = player.get_single_mut() {
            if let Some(animation) = scene_handle.animations.first() {
                player.play(animation.clone_weak()).repeat();
                *done = true;
            }
        }
    }
}
fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<&mut AnimationPlayer>,
    scene_handle: Res<SceneHandle>,
    mut current_animation: Local<usize>,
    mut changing: Local<bool>,
) {
    if scene_handle.animations.is_empty() {
        return;
    }

    if let Ok(mut player) = animation_player.get_single_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                player.resume();
            } else {
                player.pause();
            }
        }

        if *changing {
            // change the animation the frame after return was pressed
            *current_animation = (*current_animation + 1) % scene_handle.animations.len();
            player
                .play(scene_handle.animations[*current_animation].clone_weak())
                .repeat();
            *changing = false;
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            // delay the animation change for one frame
            *changing = true;
            // set the current animation to its start and pause it to reset to its starting state
            player.set_elapsed(0.0).pause();
        }
    }
}

fn keyboard_cameras_control(
    mut cameras: Local<(bool, Vec<Entity>)>,
    mut active_camera: Local<Option<usize>>, // `None` means user-controlled camera

    scene_handle: Res<SceneHandle>,
    keyboard_input: Res<Input<KeyCode>>,
    camera_query: Query<Entity, (With<Camera3d>, Without<CameraController>)>,
    user_camera_query: Query<Entity, (With<Camera3d>, With<CameraController>)>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
) {
    if !scene_handle.is_loaded {
        return;
    }
    if !cameras.0 {
        cameras.1 = camera_query.iter().collect();
        cameras.0 = true;
    }

    if keyboard_input.just_pressed(KeyCode::C) && !cameras.1.is_empty() {
        *active_camera = match *active_camera {
            Some(index) if index + 1 == cameras.1.len() => None,
            Some(index) => Some(index + 1),
            None => Some(0),
        };

        match *active_camera {
            Some(i) => {
                info!("Using camera {i}");
                active_camera_3d.set(cameras.1[i]);
            }
            None => {
                info!("Using user-controller camera");
                active_camera_3d.set(user_camera_query.single());
            }
        }
    }
}

fn camera_spawn_check(
    mut commands: Commands,
    mut scene_handle: ResMut<SceneHandle>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Handle<Mesh>>>,
    cameras_3d: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
) {
    // If the scene did not contain a camera, find an approximate bounding box of the scene from
    // its meshes and spawn a camera that fits it in view
    if scene_handle.is_loaded && (!scene_handle.has_camera || !scene_handle.has_light) {
        if meshes.iter().any(|(_, maybe_aabb)| maybe_aabb.is_none()) {
            return;
        }

        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for (transform, maybe_aabb) in meshes.iter() {
            let aabb = maybe_aabb.unwrap();
            // If the Aabb had not been rotated, applying the non-uniform scale would produce the
            // correct bounds. However, it could very well be rotated and so we first convert to
            // a Sphere, and then back to an Aabb to find the conservative min and max points.
            let sphere = Sphere {
                center: Vec3A::from(transform.mul_vec3(Vec3::from(aabb.center))),
                radius: (Vec3A::from(transform.scale) * aabb.half_extents).length(),
            };
            let aabb = Aabb::from(sphere);
            min = min.min(aabb.min());
            max = max.max(aabb.max());
        }

        let size = (max - min).length();
        let aabb = Aabb::from_min_max(Vec3::from(min), Vec3::from(max));

        if !scene_handle.has_camera {
            let bundle = if let Some((transform, camera)) = cameras_3d.iter().next() {
                let mut transform: Transform = (*transform).into();
                let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
                transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                PerspectiveCameraBundle {
                    camera: camera.clone(),
                    transform,
                    ..PerspectiveCameraBundle::new_3d()
                }
            } else {
                let transform = Transform::from_translation(
                    Vec3::from(aabb.center) + size * Vec3::new(0.5, 0.25, 0.5),
                )
                .looking_at(Vec3::from(aabb.center), Vec3::Y);
                let view = transform.compute_matrix();
                let mut perspective_projection = PerspectiveProjection::default();
                perspective_projection.far = perspective_projection.far.max(size * 10.0);
                let view_projection =
                    view.inverse() * perspective_projection.get_projection_matrix();
                let frustum = Frustum::from_view_projection(
                    &view_projection,
                    &transform.translation,
                    &transform.back(),
                    perspective_projection.far(),
                );
                let camera = Camera {
                    near: perspective_projection.near,
                    far: perspective_projection.far,
                    ..default()
                };
                PerspectiveCameraBundle {
                    camera,
                    perspective_projection,
                    frustum,
                    transform,
                    ..PerspectiveCameraBundle::new_3d()
                }
            };

            info!("Spawning a 3D perspective camera");
            let entity = commands
                .spawn_bundle(bundle)
                .insert(CameraController::default())
                .id();
            active_camera_3d.set(entity);

            scene_handle.has_camera = true;
        }

        if !scene_handle.has_light {
            // The same approach as above but now for the scene
            let sphere = Sphere {
                center: aabb.center,
                radius: aabb.half_extents.length(),
            };
            let aabb = Aabb::from(sphere);
            let min = aabb.min();
            let max = aabb.max();

            info!("Spawning a directional light");
            commands.spawn_bundle(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadow_projection: OrthographicProjection {
                        left: min.x,
                        right: max.x,
                        bottom: min.y,
                        top: max.y,
                        near: min.z,
                        far: max.z,
                        ..default()
                    },
                    shadows_enabled: false,
                    ..default()
                },
                ..default()
            });

            scene_handle.has_light = true;
        }
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
                time.seconds_since_startup() as f32 * std::f32::consts::TAU / 30.0,
                -std::f32::consts::FRAC_PI_4,
            );
        }
    }
}

#[derive(Component)]
struct CameraController {
    pub enabled: bool,
    pub initialized: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub key_enable_mouse: MouseButton,
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
            initialized: false,
            sensitivity: 0.5,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            key_enable_mouse: MouseButton::Left,
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
    mouse_button_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut transform, mut options)) = query.get_single_mut() {
        if !options.initialized {
            let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
            options.yaw = yaw;
            options.pitch = pitch;
            options.initialized = true;
        }
        if !options.enabled {
            return;
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

        // Handle mouse input
        let mut mouse_delta = Vec2::ZERO;
        if mouse_button_input.pressed(options.key_enable_mouse) {
            for mouse_event in mouse_events.iter() {
                mouse_delta += mouse_event.delta;
            }
        }

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
