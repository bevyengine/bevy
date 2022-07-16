//! A simple glTF scene viewer made with Bevy.
//!
//! Just run `cargo run --release --example scene_viewer /path/to/model.gltf#Scene0`,
//! replacing the path as appropriate.
//! With no arguments it will load the `FieldHelmet` glTF model from the repository assets subdirectory.

use bevy::{
    asset::{AssetServerSettings, LoadState},
    gltf::Gltf,
    input::mouse::MouseMotion,
    math::Vec3A,
    prelude::*,
    render::primitives::{Aabb, Sphere},
    scene::InstanceId,
};

use std::f32::consts::TAU;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
struct CameraControllerCheckSystem;

fn main() {
    println!(
        "
Controls:
    MOUSE       - Move camera orientation
    LClick/M    - Enable mouse movement
    WSAD        - forward/back/strafe left/right
    LShift      - 'run'
    E           - up
    Q           - down
    L           - animate light direction
    U           - toggle shadows
    C           - cycle through cameras
    5/6         - decrease/increase shadow projection width
    7/8         - decrease/increase shadow projection height
    9/0         - decrease/increase shadow projection near/far

    Space       - Play/Pause animation
    Enter       - Cycle through animations
"
    );
    let mut app = App::new();
    app.insert_resource(AmbientLight {
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
    .init_resource::<CameraTracker>()
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup)
    .add_system_to_stage(CoreStage::PreUpdate, scene_load_check)
    .add_system_to_stage(CoreStage::PreUpdate, setup_scene_after_load)
    .add_system(update_lights)
    .add_system(camera_controller)
    .add_system(camera_tracker);

    #[cfg(feature = "animation")]
    app.add_system(start_animation)
        .add_system(keyboard_animation_control);

    app.run();
}

struct SceneHandle {
    handle: Handle<Gltf>,
    #[cfg(feature = "animation")]
    animations: Vec<Handle<AnimationClip>>,
    instance_id: Option<InstanceId>,
    is_loaded: bool,
    has_light: bool,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/models/FlightHelmet/FlightHelmet.gltf".to_string());
    info!("Loading {}", scene_path);
    commands.insert_resource(SceneHandle {
        handle: asset_server.load(&scene_path),
        #[cfg(feature = "animation")]
        animations: Vec::new(),
        instance_id: None,
        is_loaded: false,
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

                #[cfg(feature = "animation")]
                {
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

#[cfg(feature = "animation")]
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

#[cfg(feature = "animation")]
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

fn setup_scene_after_load(
    mut commands: Commands,
    mut setup: Local<bool>,
    mut scene_handle: ResMut<SceneHandle>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Handle<Mesh>>>,
) {
    if scene_handle.is_loaded && !*setup {
        *setup = true;
        // Find an approximate bounding box of the scene from its meshes
        if meshes.iter().any(|(_, maybe_aabb)| maybe_aabb.is_none()) {
            return;
        }

        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for (transform, maybe_aabb) in &meshes {
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

        info!("Spawning a controllable 3D perspective camera");
        let mut projection = PerspectiveProjection::default();
        projection.far = projection.far.max(size * 10.0);
        commands
            .spawn_bundle(Camera3dBundle {
                projection: projection.into(),
                transform: Transform::from_translation(
                    Vec3::from(aabb.center) + size * Vec3::new(0.5, 0.25, 0.5),
                )
                .looking_at(Vec3::from(aabb.center), Vec3::Y),
                camera: Camera {
                    is_active: false,
                    ..default()
                },
                ..default()
            })
            .insert(CameraController::default());

        // Spawn a default light if the scene does not have one
        if !scene_handle.has_light {
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
    for (_, mut light) in &mut query {
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
        for (mut transform, _) in &mut query {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.seconds_since_startup() as f32 * TAU / 30.0,
                -TAU / 8.,
            );
        }
    }
}

#[derive(Default)]
struct CameraTracker {
    active_index: Option<usize>,
    cameras: Vec<Entity>,
}

impl CameraTracker {
    fn track_camera(&mut self, entity: Entity) -> bool {
        self.cameras.push(entity);
        if self.active_index.is_none() {
            self.active_index = Some(self.cameras.len() - 1);
            true
        } else {
            false
        }
    }

    fn active_camera(&self) -> Option<Entity> {
        self.active_index.map(|i| self.cameras[i])
    }

    fn set_next_active(&mut self) -> Option<Entity> {
        let active_index = self.active_index?;
        let new_i = (active_index + 1) % self.cameras.len();
        self.active_index = Some(new_i);
        Some(self.cameras[new_i])
    }
}

fn camera_tracker(
    mut camera_tracker: ResMut<CameraTracker>,
    keyboard_input: Res<Input<KeyCode>>,
    mut queries: ParamSet<(
        Query<(Entity, &mut Camera), (Added<Camera>, Without<CameraController>)>,
        Query<(Entity, &mut Camera), (Added<Camera>, With<CameraController>)>,
        Query<&mut Camera>,
    )>,
) {
    // track added scene camera entities first, to ensure they are preferred for the
    // default active camera
    for (entity, mut camera) in queries.p0().iter_mut() {
        camera.is_active = camera_tracker.track_camera(entity);
    }

    // iterate added custom camera entities second
    for (entity, mut camera) in queries.p1().iter_mut() {
        camera.is_active = camera_tracker.track_camera(entity);
    }

    if keyboard_input.just_pressed(KeyCode::C) {
        // disable currently active camera
        if let Some(e) = camera_tracker.active_camera() {
            if let Ok(mut camera) = queries.p2().get_mut(e) {
                camera.is_active = false;
            }
        }

        // enable next active camera
        if let Some(e) = camera_tracker.set_next_active() {
            if let Ok(mut camera) = queries.p2().get_mut(e) {
                camera.is_active = true;
            }
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
    pub mouse_key_enable_mouse: MouseButton,
    pub keyboard_key_enable_mouse: KeyCode,
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
            mouse_key_enable_mouse: MouseButton::Left,
            keyboard_key_enable_mouse: KeyCode::M,
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
    mut move_toggled: Local<bool>,
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
        if key_input.just_pressed(options.keyboard_key_enable_mouse) {
            *move_toggled = !*move_toggled;
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
        if mouse_button_input.pressed(options.mouse_key_enable_mouse) || *move_toggled {
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
