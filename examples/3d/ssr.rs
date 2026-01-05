//! Demonstrates screen space reflections in deferred rendering.

use std::ops::Range;

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    color::palettes::css::{BLACK, WHITE},
    core_pipeline::Skybox,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    input::mouse::MouseWheel,
    math::{vec3, vec4},
    pbr::{
        DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension,
        ScreenSpaceAmbientOcclusion, ScreenSpaceReflections,
    },
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderType},
        view::Hdr,
    },
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/water_material.wgsl";

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
static SWITCH_TO_CAPSULES_HELP_TEXT: &str = "Press Enter to switch to the row of capsules model";
static SWITCH_TO_CUBE_HELP_TEXT: &str = "Press Enter to switch to the single cube model";
static MIN_ROUGHNESS_HELP_TEXT: &str = "Press U/I and O/P to adjust the minimum roughness range";
static MAX_ROUGHNESS_HELP_TEXT: &str = "Press H/J and K/L to adjust the maximum roughness range";
static EDGE_FADEOUT_HELP_TEXT: &str = "Press N/M and ,/. to adjust the edge fadeout range";

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
    /// The perceptual roughness range over which SSR begins to fade in.
    min_perceptual_roughness: Range<f32>,
    /// The perceptual roughness range over which SSR begins to fade out.
    max_perceptual_roughness: Range<f32>,
    /// The range over which SSR begins to fade out at the edges of the screen.
    edge_fadeout: Range<f32>,
}

/// Which model is being displayed.
#[derive(Default)]
enum DisplayedModel {
    /// The cube is being displayed.
    #[default]
    Cube,
    /// The flight helmet is being displayed.
    FlightHelmet,
    /// The capsules are being displayed.
    Capsules,
}

/// A marker component for the single cube model.
#[derive(Component)]
struct CubeModel;

/// A marker component for the flight helmet model.
#[derive(Component)]
struct FlightHelmetModel;

/// A marker component for the row of capsules model.
#[derive(Component)]
struct CapsuleModel;

/// A marker component for the row of capsules parent.
#[derive(Component)]
struct CapsulesParent;

fn main() {
    // Enable deferred rendering, which is necessary for screen-space
    // reflections at this time. Disable multisampled antialiasing, as deferred
    // rendering doesn't support that.
    App::new()
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
    spawn_capsules(&mut commands, &mut meshes, &mut standard_materials);
    spawn_water(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut water_materials,
    );
    spawn_camera(&mut commands, &asset_server, &app_settings);
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
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(standard_materials.add(StandardMaterial {
                base_color: Color::from(WHITE),
                base_color_texture: Some(asset_server.load("branding/icon.png")),
                ..default()
            })),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .insert(CubeModel);
}

// Spawns the flight helmet.
fn spawn_flight_helmet(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        Transform::from_scale(Vec3::splat(2.5)),
        FlightHelmetModel,
        Visibility::Hidden,
    ));
}

// Spawns the row of capsules.
fn spawn_capsules(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    let capsule_mesh = meshes.add(Capsule3d::new(0.4, 0.5));
    let parent = commands
        .spawn((
            Transform::from_xyz(0.0, 0.5, 0.0),
            Visibility::Hidden,
            CapsulesParent,
        ))
        .id();

    for i in 0..5 {
        let roughness = i as f32 * 0.25;
        let child = commands
            .spawn((
                Mesh3d(capsule_mesh.clone()),
                MeshMaterial3d(standard_materials.add(StandardMaterial {
                    base_color: Color::BLACK,
                    perceptual_roughness: roughness,
                    ..default()
                })),
                Transform::from_xyz(i as f32 * 1.1 - (1.1 * 2.0), 0.5, 0.0),
                CapsuleModel,
            ))
            .id();
        commands.entity(parent).add_child(child);
    }
}

// Spawns the water plane.
fn spawn_water(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<ExtendedMaterial<StandardMaterial, Water>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0)))),
        MeshMaterial3d(water_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: BLACK.into(),
                perceptual_roughness: 0.09,
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
        })),
        Transform::from_scale(Vec3::splat(100.0)),
    ));
}

// Spawns the camera.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer, app_settings: &AppSettings) {
    // Create the camera. Add an environment map and skybox so the water has
    // something interesting to reflect, other than the cube. Enable deferred
    // rendering by adding depth and deferred prepasses. Turn on FXAA to make
    // the scene look a little nicer. Finally, add screen space reflections.
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(vec3(-1.25, 2.25, 4.5)).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
        Msaa::Off,
        TemporalAntiAliasing::default(),
        ScreenSpaceReflections {
            min_perceptual_roughness: app_settings.min_perceptual_roughness.clone(),
            max_perceptual_roughness: app_settings.max_perceptual_roughness.clone(),
            edge_fadeout: app_settings.edge_fadeout.clone(),
            ..default()
        },
        ScreenSpaceAmbientOcclusion::default(),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 5000.0,
            ..default()
        },
        Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 5000.0,
            ..default()
        },
    ));
}

// Spawns the help text.
fn spawn_text(commands: &mut Commands, app_settings: &AppSettings) {
    commands.spawn((
        create_text(app_settings),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// Creates or recreates the help text.
fn create_text(app_settings: &AppSettings) -> Text {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\nSSR min roughness: {:.2}..{:.2}\nSSR max roughness: {:.2}..{:.2}\nSSR edge fadeout: {:.2}..{:.2}",
        match app_settings.displayed_model {
            DisplayedModel::Cube => SWITCH_TO_FLIGHT_HELMET_HELP_TEXT,
            DisplayedModel::FlightHelmet => SWITCH_TO_CAPSULES_HELP_TEXT,
            DisplayedModel::Capsules => SWITCH_TO_CUBE_HELP_TEXT,
        },
        if app_settings.ssr_on {
            TURN_SSR_OFF_HELP_TEXT
        } else {
            TURN_SSR_ON_HELP_TEXT
        },
        MOVE_CAMERA_HELP_TEXT,
        MIN_ROUGHNESS_HELP_TEXT,
        MAX_ROUGHNESS_HELP_TEXT,
        EDGE_FADEOUT_HELP_TEXT,
        app_settings.min_perceptual_roughness.start,
        app_settings.min_perceptual_roughness.end,
        app_settings.max_perceptual_roughness.start,
        app_settings.max_perceptual_roughness.end,
        app_settings.edge_fadeout.start,
        app_settings.edge_fadeout.end,
    )
    .into()
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

/// Rotates the model on the Y axis a bit every frame.
fn rotate_model(
    mut query: Query<&mut Transform, Or<(With<CubeModel>, With<FlightHelmetModel>)>>,
    time: Res<Time>,
) {
    for mut transform in query.iter_mut() {
        // Models rotate on the Y axis.
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs());
    }
}

// Processes input related to camera movement.
fn move_camera(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
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
    for mouse_wheel in mouse_wheel_reader.read() {
        distance_delta -= mouse_wheel.y * CAMERA_MOUSE_WHEEL_ZOOM_SPEED;
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
fn adjust_app_settings(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_settings: ResMut<AppSettings>,
    mut cameras: Query<Entity, With<Camera>>,
    mut cube_models: Query<
        &mut Visibility,
        (
            With<CubeModel>,
            Without<FlightHelmetModel>,
            Without<CapsuleModel>,
            Without<CapsulesParent>,
        ),
    >,
    mut flight_helmet_models: Query<
        &mut Visibility,
        (
            Without<CubeModel>,
            With<FlightHelmetModel>,
            Without<CapsuleModel>,
            Without<CapsulesParent>,
        ),
    >,
    mut capsules_row_models: Query<
        &mut Visibility,
        (
            Without<CubeModel>,
            Without<FlightHelmetModel>,
            Or<(With<CapsuleModel>, With<CapsulesParent>)>,
        ),
    >,
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
            DisplayedModel::FlightHelmet => DisplayedModel::Capsules,
            DisplayedModel::Capsules => DisplayedModel::Cube,
        };
        any_changes = true;
    }

    // Adjust min roughness range.
    if keyboard_input.pressed(KeyCode::KeyU) {
        app_settings.min_perceptual_roughness.start =
            (app_settings.min_perceptual_roughness.start - 0.01).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyI) {
        app_settings.min_perceptual_roughness.start =
            (app_settings.min_perceptual_roughness.start + 0.01).min(1.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyO) {
        app_settings.min_perceptual_roughness.end =
            (app_settings.min_perceptual_roughness.end - 0.01).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyP) {
        app_settings.min_perceptual_roughness.end =
            (app_settings.min_perceptual_roughness.end + 0.01).min(1.0);
        any_changes = true;
    }

    // Adjust max roughness range.
    if keyboard_input.pressed(KeyCode::KeyH) {
        app_settings.max_perceptual_roughness.start =
            (app_settings.max_perceptual_roughness.start - 0.01).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyJ) {
        app_settings.max_perceptual_roughness.start =
            (app_settings.max_perceptual_roughness.start + 0.01).min(1.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyK) {
        app_settings.max_perceptual_roughness.end =
            (app_settings.max_perceptual_roughness.end - 0.01).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyL) {
        app_settings.max_perceptual_roughness.end =
            (app_settings.max_perceptual_roughness.end + 0.01).min(1.0);
        any_changes = true;
    }

    // Adjust edge fadeout range.
    if keyboard_input.pressed(KeyCode::KeyN) {
        app_settings.edge_fadeout.start = (app_settings.edge_fadeout.start - 0.001).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::KeyM) {
        app_settings.edge_fadeout.start = (app_settings.edge_fadeout.start + 0.001).min(1.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::Comma) {
        app_settings.edge_fadeout.end = (app_settings.edge_fadeout.end - 0.001).max(0.0);
        any_changes = true;
    }
    if keyboard_input.pressed(KeyCode::Period) {
        app_settings.edge_fadeout.end = (app_settings.edge_fadeout.end + 0.001).min(1.0);
        any_changes = true;
    }

    // If there were no changes, bail.
    if !any_changes {
        return;
    }

    // Update SSR settings.
    for camera in cameras.iter_mut() {
        if app_settings.ssr_on {
            commands.entity(camera).insert(ScreenSpaceReflections {
                min_perceptual_roughness: app_settings.min_perceptual_roughness.clone(),
                max_perceptual_roughness: app_settings.max_perceptual_roughness.clone(),
                edge_fadeout: app_settings.edge_fadeout.clone(),
                ..default()
            });
        } else {
            commands.entity(camera).remove::<ScreenSpaceReflections>();
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

    // Set row of capsules model visibility.
    for mut capsules_row_visibility in capsules_row_models.iter_mut() {
        *capsules_row_visibility = match app_settings.displayed_model {
            DisplayedModel::Capsules => Visibility::Visible,
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
            min_perceptual_roughness: 0.0..0.0,
            max_perceptual_roughness: 0.55..0.7,
            edge_fadeout: 0.0..0.0,
        }
    }
}
