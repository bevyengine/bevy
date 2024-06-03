//! Demonstrates how to enable per-object motion blur. This rendering feature can be configured per
//! camera using the [`MotionBlur`] component.z

use bevy::{
    core_pipeline::motion_blur::{MotionBlur, MotionBlurBundle},
    prelude::*,
};

fn main() {
    let mut app = App::new();

    // MSAA and Motion Blur together are not compatible on WebGL
    #[cfg(all(feature = "webgl2", target_arch = "wasm32", not(feature = "webgpu")))]
    app.insert_resource(Msaa::Off);

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, setup_scene, setup_ui))
        .add_systems(Update, (keyboard_inputs, move_cars, move_camera).chain())
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        // Add the MotionBlurBundle to a camera to enable motion blur.
        // Motion blur requires the depth and motion vector prepass, which this bundle adds.
        // Configure the amount and quality of motion blur per-camera using this component.
        MotionBlurBundle {
            motion_blur: MotionBlur {
                shutter_angle: 1.0,
                samples: 2,
                #[cfg(all(feature = "webgl2", target_arch = "wasm32", not(feature = "webgpu")))]
                _webgl2_padding: Default::default(),
            },
            ..default()
        },
    ));
}

// Everything past this point is used to build the example, but isn't required to use motion blur.

#[derive(Resource)]
enum CameraMode {
    Track,
    Chase,
}

#[derive(Component)]
struct Moves(f32);

#[derive(Component)]
struct CameraTracked;

#[derive(Component)]
struct Rotates;

fn setup_scene(
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });
    commands.insert_resource(CameraMode::Chase);
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 3_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::default().looking_to(Vec3::new(-1.0, -0.7, -1.0), Vec3::X),
        ..default()
    });
    // Sky
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::default()),
        material: materials.add(StandardMaterial {
            unlit: true,
            base_color: Color::linear_rgb(0.1, 0.6, 1.0),
            ..default()
        }),
        transform: Transform::default().with_scale(Vec3::splat(-4000.0)),
        ..default()
    });
    // Ground
    let mut plane: Mesh = Plane3d::default().into();
    let uv_size = 4000.0;
    let uvs = vec![[uv_size, 0.0], [0.0, 0.0], [0.0, uv_size], [uv_size; 2]];
    plane.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    commands.spawn(PbrBundle {
        mesh: meshes.add(plane),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -0.65, 0.0).with_scale(Vec3::splat(80.)),
        ..default()
    });

    spawn_cars(&asset_server, &mut meshes, &mut materials, &mut commands);
    spawn_trees(&mut meshes, &mut materials, &mut commands);
    spawn_barriers(&mut meshes, &mut materials, &mut commands);
}

fn spawn_cars(
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) {
    const N_CARS: usize = 20;
    let box_mesh = meshes.add(Cuboid::new(0.3, 0.15, 0.55));
    let cylinder = meshes.add(Cylinder::default());
    let logo = asset_server.load("branding/icon.png");
    let wheel_matl = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(logo.clone()),
        ..default()
    });

    let mut matl = |color| {
        materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })
    };

    let colors = [
        matl(Color::linear_rgb(1.0, 0.0, 0.0)),
        matl(Color::linear_rgb(1.0, 1.0, 0.0)),
        matl(Color::BLACK),
        matl(Color::linear_rgb(0.0, 0.0, 1.0)),
        matl(Color::linear_rgb(0.0, 1.0, 0.0)),
        matl(Color::linear_rgb(1.0, 0.0, 1.0)),
        matl(Color::linear_rgb(0.5, 0.5, 0.0)),
        matl(Color::linear_rgb(1.0, 0.5, 0.0)),
    ];

    for i in 0..N_CARS {
        let color = colors[i % colors.len()].clone();
        let mut entity = commands.spawn((
            PbrBundle {
                mesh: box_mesh.clone(),
                material: color.clone(),
                transform: Transform::from_scale(Vec3::splat(0.5)),
                ..default()
            },
            Moves(i as f32 * 2.0),
        ));
        if i == 0 {
            entity.insert(CameraTracked);
        }
        entity.with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh: box_mesh.clone(),
                material: color,
                transform: Transform::from_xyz(0.0, 0.08, 0.03)
                    .with_scale(Vec3::new(1.0, 1.0, 0.5)),
                ..default()
            });
            let mut spawn_wheel = |x: f32, z: f32| {
                parent.spawn((
                    PbrBundle {
                        mesh: cylinder.clone(),
                        material: wheel_matl.clone(),
                        transform: Transform::from_xyz(0.14 * x, -0.045, 0.15 * z)
                            .with_scale(Vec3::new(0.15, 0.04, 0.15))
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        ..default()
                    },
                    Rotates,
                ));
            };
            spawn_wheel(1.0, 1.0);
            spawn_wheel(1.0, -1.0);
            spawn_wheel(-1.0, 1.0);
            spawn_wheel(-1.0, -1.0);
        });
    }
}

fn spawn_barriers(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) {
    const N_CONES: usize = 100;
    let capsule = meshes.add(Capsule3d::default());
    let matl = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(255, 87, 51),
        reflectance: 1.0,
        ..default()
    });
    let mut spawn_with_offset = |offset: f32| {
        for i in 0..N_CONES {
            let pos = race_track_pos(
                offset,
                (i as f32) / (N_CONES as f32) * std::f32::consts::PI * 2.0,
            );
            commands.spawn(PbrBundle {
                mesh: capsule.clone(),
                material: matl.clone(),
                transform: Transform::from_xyz(pos.x, -0.65, pos.y).with_scale(Vec3::splat(0.07)),
                ..default()
            });
        }
    };
    spawn_with_offset(0.04);
    spawn_with_offset(-0.04);
}

fn spawn_trees(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) {
    const N_TREES: usize = 30;
    let capsule = meshes.add(Capsule3d::default());
    let sphere = meshes.add(Sphere::default());
    let leaves = materials.add(Color::linear_rgb(0.0, 1.0, 0.0));
    let trunk = materials.add(Color::linear_rgb(0.4, 0.2, 0.2));

    let mut spawn_with_offset = |offset: f32| {
        for i in 0..N_TREES {
            let pos = race_track_pos(
                offset,
                (i as f32) / (N_TREES as f32) * std::f32::consts::PI * 2.0,
            );
            let [x, z] = pos.into();
            commands.spawn(PbrBundle {
                mesh: sphere.clone(),
                material: leaves.clone(),
                transform: Transform::from_xyz(x, -0.3, z).with_scale(Vec3::splat(0.3)),
                ..default()
            });
            commands.spawn(PbrBundle {
                mesh: capsule.clone(),
                material: trunk.clone(),
                transform: Transform::from_xyz(x, -0.5, z).with_scale(Vec3::new(0.05, 0.3, 0.05)),
                ..default()
            });
        }
    };
    spawn_with_offset(0.07);
    spawn_with_offset(-0.07);
}

fn setup_ui(mut commands: Commands) {
    let style = TextStyle::default();

    commands.spawn(
        TextBundle::from_sections(vec![
            TextSection::new(String::new(), style.clone()),
            TextSection::new(String::new(), style.clone()),
            TextSection::new("1/2: -/+ shutter angle (blur amount)\n", style.clone()),
            TextSection::new("3/4: -/+ sample count (blur quality)\n", style.clone()),
            TextSection::new("Spacebar: cycle camera\n", style.clone()),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn keyboard_inputs(
    mut settings: Query<&mut MotionBlur>,
    presses: Res<ButtonInput<KeyCode>>,
    mut text: Query<&mut Text>,
    mut camera: ResMut<CameraMode>,
) {
    let mut settings = settings.single_mut();
    if presses.just_pressed(KeyCode::Digit1) {
        settings.shutter_angle -= 0.25;
    } else if presses.just_pressed(KeyCode::Digit2) {
        settings.shutter_angle += 0.25;
    } else if presses.just_pressed(KeyCode::Digit3) {
        settings.samples = settings.samples.saturating_sub(1);
    } else if presses.just_pressed(KeyCode::Digit4) {
        settings.samples += 1;
    } else if presses.just_pressed(KeyCode::Space) {
        *camera = match *camera {
            CameraMode::Track => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Track,
        };
    }
    settings.shutter_angle = settings.shutter_angle.clamp(0.0, 1.0);
    settings.samples = settings.samples.clamp(0, 64);
    let mut text = text.single_mut();
    text.sections[0].value = format!("Shutter angle: {:.2}\n", settings.shutter_angle);
    text.sections[1].value = format!("Samples: {:.5}\n", settings.samples);
}

/// Parametric function for a looping race track. `offset` will return the point offset
/// perpendicular to the track at the given point.
fn race_track_pos(offset: f32, t: f32) -> Vec2 {
    let x_tweak = 2.0;
    let y_tweak = 3.0;
    let scale = 8.0;
    let x0 = (x_tweak * t).sin();
    let y0 = (y_tweak * t).cos();
    let dx = x_tweak * (x_tweak * t).cos();
    let dy = y_tweak * -(y_tweak * t).sin();
    let x = x0 + offset * dy / (dx.powi(2) + dy.powi(2)).sqrt();
    let y = y0 - offset * dx / (dx.powi(2) + dy.powi(2)).sqrt();
    Vec2::new(x, y) * scale
}

fn move_cars(
    time: Res<Time>,
    mut movables: Query<(&mut Transform, &Moves, &Children)>,
    mut spins: Query<&mut Transform, (Without<Moves>, With<Rotates>)>,
) {
    for (mut transform, moves, children) in &mut movables {
        let time = time.elapsed_seconds() * 0.25;
        let t = time + 0.5 * moves.0;
        let dx = t.cos();
        let dz = -(3.0 * t).sin();
        let speed_variation = (dx * dx + dz * dz).sqrt() * 0.15;
        let t = t + speed_variation;
        let prev = transform.translation;
        transform.translation.x = race_track_pos(0.0, t).x;
        transform.translation.z = race_track_pos(0.0, t).y;
        transform.translation.y = -0.59;
        let delta = transform.translation - prev;
        transform.look_to(delta, Vec3::Y);
        for child in children.iter() {
            let Ok(mut wheel) = spins.get_mut(*child) else {
                continue;
            };
            let radius = wheel.scale.x;
            let circumference = 2.0 * std::f32::consts::PI * radius;
            let angle = delta.length() / circumference * std::f32::consts::PI * 2.0;
            wheel.rotate_local_y(angle);
        }
    }
}

fn move_camera(
    mut camera: Query<(&mut Transform, &mut Projection), Without<CameraTracked>>,
    tracked: Query<&Transform, With<CameraTracked>>,
    mode: Res<CameraMode>,
) {
    let tracked = tracked.single();
    let (mut transform, mut projection) = camera.single_mut();
    match *mode {
        CameraMode::Track => {
            transform.look_at(tracked.translation, Vec3::Y);
            transform.translation = Vec3::new(15.0, -0.5, 0.0);
            if let Projection::Perspective(perspective) = &mut *projection {
                perspective.fov = 0.05;
            }
        }
        CameraMode::Chase => {
            transform.translation =
                tracked.translation + Vec3::new(0.0, 0.15, 0.0) + tracked.back() * 0.6;
            transform.look_to(*tracked.forward(), Vec3::Y);
            if let Projection::Perspective(perspective) = &mut *projection {
                perspective.fov = 1.0;
            }
        }
    }
}

fn uv_debug_texture() -> Image {
    use bevy::render::{render_asset::RenderAssetUsages, render_resource::*, texture::*};
    const TEXTURE_SIZE: usize = 7;

    let mut palette = [
        164, 164, 164, 255, 168, 168, 168, 255, 153, 153, 153, 255, 139, 139, 139, 255, 153, 153,
        153, 255, 177, 177, 177, 255, 159, 159, 159, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(12);
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::MirrorRepeat,
        mag_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::linear()
    });
    img
}
