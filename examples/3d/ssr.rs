//! Demonstrates screen space reflections in deferred rendering.

use std::ops::Range;

use bevy::{
    color::palettes::css::{BLACK, WHITE},
    core_pipeline::{
        fxaa::Fxaa,
        prepass::{DeferredPrepass, DepthPrepass},
        Skybox,
    },
    input::mouse::MouseWheel,
    math::{vec3, vec4},
    pbr::{
        DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension, ScreenSpaceReflections,
    },
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderRef, ShaderType},
        texture::{
            ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
            ImageSamplerDescriptor,
        },
    },
};

// The speed of camera movement.
const CAMERA_KEYBOARD_ZOOM_SPEED: f32 = 0.1;
const CAMERA_KEYBOARD_ORBIT_SPEED: f32 = 0.02;
const CAMERA_MOUSE_WHEEL_ZOOM_SPEED: f32 = 0.25;

// We clamp camera distances to this range.
const CAMERA_ZOOM_RANGE: Range<f32> = 2.0..12.0;

static TURN_SSR_OFF_HELP_TEXT: &str = "Press Space to turn screen-space reflections off";
static TURN_SSR_ON_HELP_TEXT: &str = "Press Space to turn screen-space reflections on";
static MOVE_CAMERA_HELP_TEXT: &str =
    "Press WASD or use the mouse wheel to pan and orbit the camera";

/// A custom [`ExtendedMaterial`] that creates animated water ripples.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct Water {
    /// The normal map image.
    ///
    /// Note that, like all normal maps, this must not be loaded as sRGB.
    #[texture(100)]
    #[sampler(101)]
    normals: Handle<Image>,

    // Parameters to the water shader.
    #[uniform(102)]
    settings: WaterSettings,
}

/// Parameters to the water shader.
#[derive(ShaderType, Debug, Clone)]
struct WaterSettings {
    /// How much to displace each octave each frame, in the u and v directions.
    /// Two octaves are packed into each `vec4`.
    octave_vectors: [Vec4; 2],
    /// How wide the waves are in each octave.
    octave_scales: Vec4,
    /// How high the waves are in each octave.
    octave_strengths: Vec4,
}

/// A marker component for the rotating cube.
#[derive(Component)]
struct RotatingCube;

fn main() {
    // Enable deferred rendering, which is necessary for screen-space
    // reflections at this time. Disable multisampled antialiasing, as deferred
    // rendering doesn't support that.
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, Water>>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, move_camera)
        .add_systems(Update, toggle_ssr)
        .run();
}

// Set up the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, Water>>>,
    asset_server: Res<AssetServer>,
) {
    spawn_cube(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut standard_materials,
    );
    spawn_water(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut water_materials,
    );
    spawn_camera(&mut commands, &asset_server);
    spawn_text(&mut commands, &asset_server);
}

// Spawns the rotating cube.
fn spawn_cube(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: standard_materials.add(StandardMaterial {
                base_color: Color::from(WHITE),
                base_color_texture: Some(asset_server.load("branding/icon.png")),
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(RotatingCube);
}

// Spawns the water plane.
fn spawn_water(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<ExtendedMaterial<StandardMaterial, Water>>,
) {
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y)),
        material: water_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: BLACK.into(),
                perceptual_roughness: 0.0,
                ..default()
            },
            extension: Water {
                normals: asset_server.load_with_settings::<Image, ImageLoaderSettings>(
                    "textures/water_normals.png",
                    |settings| {
                        settings.is_srgb = false;
                        settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::Repeat,
                            mag_filter: ImageFilterMode::Linear,
                            min_filter: ImageFilterMode::Linear,
                            ..default()
                        });
                    },
                ),
                settings: WaterSettings {
                    octave_vectors: [
                        vec4(0.080, 0.059, 0.073, -0.062),
                        vec4(0.153, 0.138, -0.149, -0.195),
                    ],
                    octave_scales: vec4(1.0, 2.1, 7.9, 14.9) * 5.0,
                    octave_strengths: vec4(0.16, 0.18, 0.093, 0.044),
                },
            },
        }),
        transform: Transform::from_scale(Vec3::splat(100.0)),
        ..default()
    });
}

// Spawns the camera.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    // Create the camera. Add an environment map and skybox so the water has
    // something interesting to reflect, other than the cube. Enable deferred
    // rendering by adding depth and deferred prepasses. Turn on FXAA to make
    // the scene look a little nicer. Finally, add screen space reflections.
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(vec3(-1.25, 2.25, 4.5))
                .looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 5000.0,
        })
        .insert(Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 5000.0,
        })
        .insert(DepthPrepass)
        .insert(DeferredPrepass)
        .insert(Fxaa::default())
        .insert(create_ssr());
}

// Spawns the help text.
fn spawn_text(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(
        TextBundle {
            text: create_text(asset_server, true),
            ..TextBundle::default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

/// Creates or recreates the [`ScreenSpaceReflections`] component.
fn create_ssr() -> ScreenSpaceReflections {
    ScreenSpaceReflections {
        // The water's roughness is below this threshold, so SSR will affect it.
        perceptual_roughness_threshold: 0.1,
        // Approximate thickness of objects in the scene.
        thickness: 1.0,
        // Tuned step vectors. You need to adjust these for your scene to
        // balance quality and performance.
        major_step_count: 32,
        minor_step_count: 8,
    }
}

// Creates or recreates the help text.
fn create_text(asset_server: &AssetServer, has_ssr: bool) -> Text {
    Text::from_section(
        format!(
            "{}\n{}",
            if has_ssr {
                TURN_SSR_OFF_HELP_TEXT
            } else {
                TURN_SSR_ON_HELP_TEXT
            },
            MOVE_CAMERA_HELP_TEXT
        ),
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 24.0,
            ..default()
        },
    )
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
}

/// Rotates the cube on the Y axis a bit every frame.
fn rotate_cube(mut query: Query<&mut Transform, With<RotatingCube>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform.rotation =
            Quat::from_euler(EulerRot::XYZ, 0.0, time.elapsed_seconds() * 1.0, 0.0);
    }
}

// Processes input related to camera movement.
fn move_camera(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_input: EventReader<MouseWheel>,
    mut cameras: Query<&mut Transform, With<Camera>>,
) {
    let (mut distance_delta, mut theta_delta) = (0.0, 0.0);

    // Handle keyboard events.
    if keyboard_input.pressed(KeyCode::KeyW) {
        distance_delta -= CAMERA_KEYBOARD_ZOOM_SPEED;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        distance_delta += CAMERA_KEYBOARD_ZOOM_SPEED;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        theta_delta += CAMERA_KEYBOARD_ORBIT_SPEED;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        theta_delta -= CAMERA_KEYBOARD_ORBIT_SPEED;
    }

    // Handle mouse events.
    for mouse_wheel_event in mouse_wheel_input.read() {
        distance_delta -= mouse_wheel_event.y * CAMERA_MOUSE_WHEEL_ZOOM_SPEED;
    }

    // Update transforms.
    for mut camera_transform in cameras.iter_mut() {
        let local_z = camera_transform.local_z().as_vec3().normalize_or_zero();
        if distance_delta != 0.0 {
            camera_transform.translation = (camera_transform.translation.length() + distance_delta)
                .clamp(CAMERA_ZOOM_RANGE.start, CAMERA_ZOOM_RANGE.end)
                * local_z;
        }
        if theta_delta != 0.0 {
            camera_transform
                .translate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, theta_delta));
            camera_transform.look_at(Vec3::ZERO, Vec3::Y);
        }
    }
}

// Turns SSR on and off per user input.
fn toggle_ssr(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cameras: Query<(Entity, Has<ScreenSpaceReflections>), With<Camera>>,
    mut text: Query<&mut Text>,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) {
        return;
    }

    for (camera, has_ssr) in cameras.iter_mut() {
        if has_ssr {
            commands.entity(camera).remove::<ScreenSpaceReflections>();
        } else {
            commands.entity(camera).insert(create_ssr());
        }

        for mut text in text.iter_mut() {
            *text = create_text(&asset_server, !has_ssr);
        }
    }
}
