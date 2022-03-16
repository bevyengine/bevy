use bevy::{input::mouse::MouseMotion, prelude::*};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(camera_controller)
        .run();
}

/// A test for shadow cubemaps. View the cubemap faces in RenderDoc/Xcode.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // a point light with shadows at the origin
    commands
        .spawn_bundle(PointLightBundle {
            point_light: PointLight {
                intensity: 150.0,
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere::default())),
                material: materials.add(Color::FUCHSIA.into()),
                transform: Transform::from_scale(Vec3::splat(0.05)),
                ..Default::default()
            });
        });

    let beige = materials.add(Color::BEIGE.into());
    let white = materials.add(Color::WHITE.into());

    let small = 0.25;
    let big = 2.5;

    for [x, y, z, bx, by, bz] in [
        [1.0, 0.0, 0.25, 0.1, 1.0, 1.0],
        [-1.0, 0.0, 0.25, 0.1, 1.0, 1.0],
        [0.0, 1.0, 0.25, 1.0, 0.1, 1.0],
        [0.0, -1.0, 0.25, 1.0, 0.1, 1.0],
        [0.25, 0.0, 1.0, 1.0, 1.0, 0.1],
        [0.25, 0.0, -1.0, 1.0, 1.0, 0.1],
    ] {
        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(shape::Box::new(bx * small, by * small, bz * small).into()),
            transform: Transform::from_xyz(x, y, z),
            material: beige.clone(),
            ..Default::default()
        });
        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(shape::Box::new(bx * big, by * big, bz * big).into()),
            transform: Transform::from_xyz(1.5 * x, 1.5 * y, 1.5 * z),
            material: white.clone(),
            ..Default::default()
        });
    }

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-0.25, 0.0, 0.75).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(CameraController::default());
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
            walk_speed: 2.0,
            run_speed: 6.0,
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
    mut next_print: Local<f64>,
) {
    let dt = time.delta_seconds();
    let t = time.seconds_since_startup();
    let should_print = if t > *next_print {
        *next_print += 2.0;
        true
    } else {
        false
    };

    // Handle mouse input
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.iter() {
        mouse_delta += mouse_event.delta;
    }

    for (mut transform, mut options) in query.iter_mut() {
        if should_print {
            println!("Forward: {:?}", transform.forward());
        }

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
