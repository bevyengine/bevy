//! Demonstrates screen space reflections in deferred rendering.

use std::ops::Range;

use bevy::{
    color::palettes::css::{BLACK, WHITE},
    core_pipeline::{fxaa::Fxaa, Skybox},
    input::mouse::MouseWheel,
    math::{vec3, vec4},
    pbr::{
        DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension,
        ScreenSpaceReflectionsBundle, ScreenSpaceReflectionsSettings,
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
static SWITCH_TO_FLIGHT_HELMET_HELP_TEXT: &str = "Press Enter to switch to the flight helmet model";
static SWITCH_TO_CUBE_HELP_TEXT: &str = "Press Enter to switch to the cube model";

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

/// The current settings that the user has chosen.
#[derive(Resource)]
struct AppSettings {
    /// Whether screen space reflections are on.
    ssr_on: bool,
    /// Which model is being displayed.
    displayed_model: DisplayedModel,
}

/// Which model is being displayed.
#[derive(Default)]
enum DisplayedModel {
    /// The cube is being displayed.
    #[default]
    Cube,
    /// The flight helmet is being displayed.
    FlightHelmet,
}

/// A marker component for the cube model.
#[derive(Component)]
struct CubeModel;

/// A marker component for the flight helmet model.
#[derive(Component)]
struct FlightHelmetModel;

fn main() {
    // Enable deferred rendering, which is necessary for screen-space
    // reflections at this time. Disable multisampled antialiasing, as deferred
    // rendering doesn't support that.
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .init_resource::<AppSettings>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Screen Space Reflections Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, Water>>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_model)
        .add_systems(Update, move_camera)
        .add_systems(Update, adjust_app_settings)
        .run();
}

// Set up the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, Water>>>,
    asset_server: Res<AssetServer>,
    app_settings: Res<AppSettings>,
) {
    spawn_cube(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut standard_materials,
    );
    spawn_flight_helmet(&mut commands, &asset_server);
    spawn_water(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut water_materials,
    );
    spawn_camera(&mut commands, &asset_server);
    spawn_text(&mut commands, &app_settings);
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
        .insert(CubeModel);
}

// Spawns the flight helmet.
fn spawn_flight_helmet(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(SceneBundle {
            scene: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
            transform: Transform::from_scale(Vec3::splat(2.5)),
            ..default()
        })
        .insert(FlightHelmetModel)
        .insert(Visibility::Hidden);
}

// Spawns the water plane.
fn spawn_water(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<ExtendedMaterial<StandardMaterial, Water>>,
) {
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0))),
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
                // These water settings are just random values to create some
                // variety.
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
        .insert(ScreenSpaceReflectionsBundle::default())
        .insert(Fxaa::default());
}

// Spawns the help text.
fn spawn_text(commands: &mut Commands, app_settings: &AppSettings) {
    commands.spawn(
        TextBundle {
            text: create_text(app_settings),
            ..default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

// Creates or recreates the help text.
fn create_text(app_settings: &AppSettings) -> Text {
    Text::from_section(
        format!(
            "{}\n{}\n{}",
            match app_settings.displayed_model {
                DisplayedModel::Cube => SWITCH_TO_FLIGHT_HELMET_HELP_TEXT,
                DisplayedModel::FlightHelmet => SWITCH_TO_CUBE_HELP_TEXT,
            },
            if app_settings.ssr_on {
                TURN_SSR_OFF_HELP_TEXT
            } else {
                TURN_SSR_ON_HELP_TEXT
            },
            MOVE_CAMERA_HELP_TEXT
        ),
        TextStyle::default(),
    )
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
}

/// Rotates the model on the Y axis a bit every frame.
fn rotate_model(
    mut query: Query<&mut Transform, Or<(With<CubeModel>, With<FlightHelmetModel>)>>,
    time: Res<Time>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.0, time.elapsed_seconds(), 0.0);
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

// Adjusts app settings per user input.
#[allow(clippy::too_many_arguments)]
fn adjust_app_settings(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_settings: ResMut<AppSettings>,
    mut cameras: Query<Entity, With<Camera>>,
    mut cube_models: Query<&mut Visibility, (With<CubeModel>, Without<FlightHelmetModel>)>,
    mut flight_helmet_models: Query<&mut Visibility, (Without<CubeModel>, With<FlightHelmetModel>)>,
    mut text: Query<&mut Text>,
) {
    // If there are no changes, we're going to bail for efficiency. Record that
    // here.
    let mut any_changes = false;

    // If the user pressed Space, toggle SSR.
    if keyboard_input.just_pressed(KeyCode::Space) {
        app_settings.ssr_on = !app_settings.ssr_on;
        any_changes = true;
    }

    // If the user pressed Enter, switch models.
    if keyboard_input.just_pressed(KeyCode::Enter) {
        app_settings.displayed_model = match app_settings.displayed_model {
            DisplayedModel::Cube => DisplayedModel::FlightHelmet,
            DisplayedModel::FlightHelmet => DisplayedModel::Cube,
        };
        any_changes = true;
    }

    // If there were no changes, bail.
    if !any_changes {
        return;
    }

    // Update SSR settings.
    for camera in cameras.iter_mut() {
        if app_settings.ssr_on {
            commands
                .entity(camera)
                .insert(ScreenSpaceReflectionsSettings::default());
        } else {
            commands
                .entity(camera)
                .remove::<ScreenSpaceReflectionsSettings>();
        }
    }

    // Set cube model visibility.
    for mut cube_visibility in cube_models.iter_mut() {
        *cube_visibility = match app_settings.displayed_model {
            DisplayedModel::Cube => Visibility::Visible,
            _ => Visibility::Hidden,
        }
    }

    // Set flight helmet model visibility.
    for mut flight_helmet_visibility in flight_helmet_models.iter_mut() {
        *flight_helmet_visibility = match app_settings.displayed_model {
            DisplayedModel::FlightHelmet => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }

    // Update the help text.
    for mut text in text.iter_mut() {
        *text = create_text(&app_settings);
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ssr_on: true,
            displayed_model: default(),
        }
    }
}
