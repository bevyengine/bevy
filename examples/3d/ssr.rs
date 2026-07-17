//! Demonstrates screen space reflections in deferred rendering.

use std::fmt;
use std::ops::Range;

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Hdr,
    color::palettes::css::{BLACK, WHITE},
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    input::mouse::MouseWheel,
    light::Skybox,
    math::{vec3, vec4},
    pbr::{
        DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension,
        ScreenSpaceAmbientOcclusion, ScreenSpaceReflections,
    },
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    ui_widgets::{radio_self_update, ValueChange},
};

#[path = "../helpers/widgets.rs"]
mod widgets;

use widgets::{
    handle_ui_interactions, update_ui_radio_button_text, RadioButton, RadioButtonText,
    WidgetClickEvent, WidgetClickSender, BUTTON_BORDER, BUTTON_BORDER_COLOR,
    BUTTON_BORDER_RADIUS_SIZE, BUTTON_PADDING,
};

#[path = "../helpers/radio.rs"]
mod radio;

use radio::{feathers_option_buttons, main_ui_node_scene, ui_text_scene};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/water_material.wgsl";

// The speed of camera movement.
const CAMERA_KEYBOARD_ZOOM_SPEED: f32 = 0.1;
const CAMERA_KEYBOARD_ORBIT_SPEED: f32 = 0.02;
const CAMERA_MOUSE_WHEEL_ZOOM_SPEED: f32 = 0.25;

// We clamp camera distances to this range.
const CAMERA_ZOOM_RANGE: Range<f32> = 2.0..12.0;

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
    ssr_on: SsrOn,
    /// Which model is being displayed.
    displayed_model: DisplayedModel,
    /// Which base is being displayed.
    displayed_base: DisplayedBase,
    /// The perceptual roughness range over which SSR begins to fade in.
    min_perceptual_roughness: Range<f32>,
    /// The perceptual roughness range over which SSR begins to fade out.
    max_perceptual_roughness: Range<f32>,
    /// The range over which SSR begins to fade out at the edges of the screen.
    edge_fadeout: Range<f32>,
}

/// Whether screen space reflections are on.
#[derive(Clone, Copy, Deref, PartialEq)]
pub struct SsrOn(bool);

impl Default for SsrOn {
    fn default() -> Self {
        Self(true)
    }
}

/// Which model is being displayed.
#[derive(Default, PartialEq, Copy, Clone)]
enum DisplayedModel {
    /// The cube is being displayed.
    #[default]
    Cube,
    /// The flight helmet is being displayed.
    FlightHelmet,
    /// The capsules are being displayed.
    Capsules,
}

/// Which base is being displayed.
#[derive(Default, PartialEq, Copy, Clone)]
enum DisplayedBase {
    /// The water base is being displayed.
    #[default]
    Water,
    /// A slightly rough metallic base is being displayed.
    Metallic,
    /// A very rough non-metallic base is being displayed.
    RedPlane,
}

impl fmt::Display for DisplayedModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DisplayedModel::Cube => "Cube",
            DisplayedModel::FlightHelmet => "Flight Helmet",
            DisplayedModel::Capsules => "Capsules",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Display for DisplayedBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DisplayedBase::Water => "Water",
            DisplayedBase::Metallic => "Metallic",
            DisplayedBase::RedPlane => "Red Plane",
        };
        write!(f, "{}", name)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ExampleSetting {
    MinRoughnessStart(Adjustment),
    MinRoughnessEnd(Adjustment),
    MaxRoughnessStart(Adjustment),
    MaxRoughnessEnd(Adjustment),
    EdgeFadeoutStart(Adjustment),
    EdgeFadeoutEnd(Adjustment),
}

#[derive(Clone, Copy, PartialEq)]
enum Adjustment {
    Increase,
    Decrease,
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

/// A marker component for the metallic base.
#[derive(Component)]
struct MetallicBaseModel;

/// A marker component for the non-metallic base.
#[derive(Component)]
struct RedPlaneBaseModel;

/// A marker component for the water model.
#[derive(Component)]
struct WaterModel;

/// A marker component for the text that displays a range value.
/// The default is set so that it can be used in bsn!... temporarily?
#[derive(Component, Clone, Copy, PartialEq, Default)]
enum RangeValueText {
    #[default]
    MinRoughnessStart,
    MinRoughnessEnd,
    MaxRoughnessStart,
    MaxRoughnessEnd,
    EdgeFadeoutStart,
    EdgeFadeoutEnd,
}

#[derive(bevy::ecs::system::SystemParam)]
struct ModelQueries<'w, 's> {
    cube_models: Query<'w, 's, Entity, With<CubeModel>>,
    flight_helmet_models: Query<'w, 's, Entity, With<FlightHelmetModel>>,
    capsule_models: Query<'w, 's, Entity, Or<(With<CapsuleModel>, With<CapsulesParent>)>>,
    metallic_base_models: Query<'w, 's, Entity, With<MetallicBaseModel>>,
    non_metallic_base_models: Query<'w, 's, Entity, With<RedPlaneBaseModel>>,
    water_models: Query<'w, 's, Entity, With<WaterModel>>,
}

fn main() {
    // Enable deferred rendering, which is necessary for screen-space
    // reflections at this time. Disable multisampled antialiasing, as deferred
    // rendering doesn't support that.
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(UiTheme(create_dark_theme()))
        .init_resource::<AppSettings>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Screen Space Reflections Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
        ))
        .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, Water>>::default())
        .add_message::<WidgetClickEvent<ExampleSetting>>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_model)
        .add_systems(Update, move_camera)
        .add_systems(Update, adjust_app_settings)
        .add_systems(Update, handle_ui_interactions::<ExampleSetting>)
        .add_observer(handle_value_change_ssr_on)
        .add_observer(handle_value_change_displayed_model)
        .add_observer(handle_value_change_displayed_base)
        .add_observer(radio_self_update)
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
    spawn_metallic_base(&mut commands, &mut meshes, &mut standard_materials);
    spawn_non_metallic_base(&mut commands, &mut meshes, &mut standard_materials);
    spawn_water(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut water_materials,
    );
    spawn_camera(&mut commands, &asset_server, &app_settings);
    spawn_buttons(&mut commands, &app_settings);
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
        WorldAssetRoot(
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
                    perceptual_roughness: roughness.max(0.08),
                    ..default()
                })),
                Transform::from_xyz(i as f32 * 1.1 - (1.1 * 2.0), 0.5, 0.0),
                CapsuleModel,
            ))
            .id();
        commands.entity(parent).add_child(child);
    }
}

// Spawns the metallic base.
fn spawn_metallic_base(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0)))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::from(bevy::color::palettes::css::DARK_GRAY),
            metallic: 1.0,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(100.0)),
        MetallicBaseModel,
        Visibility::Hidden,
    ));
}

// Spawns the non-metallic base.
fn spawn_non_metallic_base(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0)))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::from(bevy::color::palettes::css::RED),
            metallic: 0.0,
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(100.0)),
        RedPlaneBaseModel,
        Visibility::Hidden,
    ));
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
        MeshMaterial3d(
            water_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: BLACK.into(),
                    perceptual_roughness: 0.09,
                    ..default()
                },
                extension: Water {
                    normals: asset_server
                        .load_builder()
                        .with_settings::<ImageLoaderSettings>(|settings| {
                            settings.is_srgb = false;
                            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                mag_filter: ImageFilterMode::Linear,
                                min_filter: ImageFilterMode::Linear,
                                ..default()
                            });
                        })
                        .load("textures/water_normals.png"),
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
        ),
        Transform::from_scale(Vec3::splat(100.0)),
        WaterModel,
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
            image: Some(asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2")),
            brightness: 5000.0,
            ..default()
        },
    ));
}

fn spawn_buttons(commands: &mut Commands, app_settings: &AppSettings) {
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children[
            feathers_option_buttons(
                "SSR",
                &[
                    (SsrOn(true), "On"),
                    (SsrOn(false), "Off"),
                ],
            ),
            feathers_option_buttons(
                "Model",
                &[
                    (DisplayedModel::Cube, "Cube"),
                    (
                        DisplayedModel::FlightHelmet,
                        "Flight Helmet",
                    ),
                    (DisplayedModel::Capsules, "Capsules"),
                ],
            ),
            feathers_option_buttons(
                "Base",
                &[
                    (DisplayedBase::Water, "Water"),
                    (DisplayedBase::Metallic, "Metallic"),
                    (DisplayedBase::RedPlane, "Red Plane"),
                ],
            ),
            range_row(
                "Min Roughness",
                app_settings.min_perceptual_roughness.start,
                app_settings.min_perceptual_roughness.end,
                RangeValueText::MinRoughnessStart,
                RangeValueText::MinRoughnessEnd,
                ExampleSetting::MinRoughnessStart(Adjustment::Decrease),
                ExampleSetting::MinRoughnessStart(Adjustment::Increase),
                ExampleSetting::MinRoughnessEnd(Adjustment::Decrease),
                ExampleSetting::MinRoughnessEnd(Adjustment::Increase),
            ),
            range_row(
                "Max Roughness",
                app_settings.max_perceptual_roughness.start,
                app_settings.max_perceptual_roughness.end,
                RangeValueText::MaxRoughnessStart,
                RangeValueText::MaxRoughnessEnd,
                ExampleSetting::MaxRoughnessStart(Adjustment::Decrease),
                ExampleSetting::MaxRoughnessStart(Adjustment::Increase),
                ExampleSetting::MaxRoughnessEnd(Adjustment::Decrease),
                ExampleSetting::MaxRoughnessEnd(Adjustment::Increase),
            ),
            range_row(
                "Edge Fadeout",
                app_settings.edge_fadeout.start,
                app_settings.edge_fadeout.end,
                RangeValueText::EdgeFadeoutStart,
                RangeValueText::EdgeFadeoutEnd,
                ExampleSetting::EdgeFadeoutStart(Adjustment::Decrease),
                ExampleSetting::EdgeFadeoutStart(Adjustment::Increase),
                ExampleSetting::EdgeFadeoutEnd(Adjustment::Decrease),
                ExampleSetting::EdgeFadeoutEnd(Adjustment::Increase),
            ),
        ]
    });
}

fn range_row(
    title: &str,
    start_value: f32,
    end_value: f32,
    start_marker: RangeValueText,
    end_marker: RangeValueText,
    start_dec: ExampleSetting,
    start_inc: ExampleSetting,
    end_dec: ExampleSetting,
    end_inc: ExampleSetting,
) -> impl Scene {
    bsn! {
        Node {
            align_items: AlignItems::Center,
        }
        Children[
            ui_text_scene(title.to_string(), Color::WHITE)
            Node {
                width: px(150),
            },

            range_controls(
                start_value,
                start_marker,
                start_dec,
                start_inc,
            ),

            ui_text_scene("to".to_string(), Color::WHITE)
            Node {
                margin: UiRect::horizontal(px(10)),
            },

            range_controls(end_value, end_marker, end_dec, end_inc),
        ]
    }
}

fn range_controls(
    value: f32,
    marker: RangeValueText,
    dec_setting: ExampleSetting,
    inc_setting: ExampleSetting,
) -> impl Scene {
    bsn! {
        Node {
            align_items: AlignItems::Center,
        }
        Children[
            adjustment_button(dec_setting, "<", Some(true)),

            Node {
                width: px(50),
                height: px(33),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {BUTTON_BORDER.with_left(px(0)).with_right(px(0))},
            }
            BackgroundColor(Color::WHITE)
            template_value(BUTTON_BORDER_COLOR)
            template_value(marker)
            Children[
                ui_text_scene(format!("{:.2}", value), Color::BLACK)
            ],

            adjustment_button(inc_setting, ">", Some(false)),
        ]
    }
}

fn adjustment_button(
    _setting: ExampleSetting,
    label: &str,
    is_left_right: Option<bool>,
) -> impl Scene {
    let border = if let Some(is_left) = is_left_right {
        if is_left {
            BUTTON_BORDER.with_right(px(0))
        } else {
            BUTTON_BORDER.with_left(px(0))
        }
    } else {
        BUTTON_BORDER
    };
    let border_radius = match is_left_right {
        Some(true) => BorderRadius::ZERO.with_left(BUTTON_BORDER_RADIUS_SIZE),
        Some(false) => BorderRadius::ZERO.with_right(BUTTON_BORDER_RADIUS_SIZE),
        None => BorderRadius::all(BUTTON_BORDER_RADIUS_SIZE),
    };

    bsn! {
        Button
        Node {
            height: px(33),
            border,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: BUTTON_PADDING,
            border_radius
        }
        template_value(BUTTON_BORDER_COLOR)
        BackgroundColor(Color::BLACK)
        RadioButton
        Children[
            ui_text_scene(label.to_string(), Color::WHITE)
            RadioButtonText
        ]
    }
}

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

/// Handles changes to the `SsrOn` radio group.
fn handle_value_change_ssr_on(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&radio::RadioButtonOptionValue<SsrOn>>,
    mut commands: Commands,
    mut app_settings: ResMut<AppSettings>,
    mut cameras: Query<Entity, With<Camera>>,
) {
    let Ok(radio::RadioButtonOptionValue(ssr_on)) = new_value_query.get(event.value) else {
        return;
    };
    app_settings.ssr_on = *ssr_on;

    for camera in cameras.iter_mut() {
        if app_settings.ssr_on.0 {
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
}

/// Handles changes to the `DisplayedModel` radio group.
fn handle_value_change_displayed_model(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&radio::RadioButtonOptionValue<DisplayedModel>>,
    mut app_settings: ResMut<AppSettings>,
    model_queries: ModelQueries,
    mut visibilities: Query<&mut Visibility>,
) {
    let Ok(radio::RadioButtonOptionValue(displayed_model)) = new_value_query.get(event.value)
    else {
        return;
    };
    app_settings.displayed_model = *displayed_model;

    for entity in model_queries.cube_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_model == DisplayedModel::Cube {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for entity in model_queries.flight_helmet_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_model == DisplayedModel::FlightHelmet {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for entity in model_queries.capsule_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_model == DisplayedModel::Capsules {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Handles changes to the `DisplayedBase` radio group.
fn handle_value_change_displayed_base(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&radio::RadioButtonOptionValue<DisplayedBase>>,
    mut app_settings: ResMut<AppSettings>,
    model_queries: ModelQueries,
    mut visibilities: Query<&mut Visibility>,
) {
    let Ok(radio::RadioButtonOptionValue(displayed_base)) = new_value_query.get(event.value) else {
        return;
    };
    app_settings.displayed_base = *displayed_base;

    for entity in model_queries.metallic_base_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_base == DisplayedBase::Metallic {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for entity in model_queries.non_metallic_base_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_base == DisplayedBase::RedPlane {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for entity in model_queries.water_models.iter() {
        if let Ok(mut visibility) = visibilities.get_mut(entity) {
            *visibility = if app_settings.displayed_base == DisplayedBase::Water {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

// Adjusts app settings per user input.
fn adjust_app_settings(
    mut app_settings: ResMut<AppSettings>,
    mut widget_click_events: MessageReader<WidgetClickEvent<ExampleSetting>>,
    mut background_colors: Query<&mut BackgroundColor>,
    radio_buttons: Query<
        (
            Entity,
            Has<BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<ExampleSetting>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    range_value_text: Query<(Entity, &RangeValueText)>,
    text_children: Query<&Children>,
    mut writer: TextUiWriter,
    text_query: Query<Entity, With<Text>>,
) {
    let mut any_changes = false;

    for event in widget_click_events.read() {
        any_changes = true;
        match **event {
            ExampleSetting::MinRoughnessStart(adj) => {
                app_settings.min_perceptual_roughness.start =
                    adjust(app_settings.min_perceptual_roughness.start, adj, 0.005);
            }
            ExampleSetting::MinRoughnessEnd(adj) => {
                app_settings.min_perceptual_roughness.end =
                    adjust(app_settings.min_perceptual_roughness.end, adj, 0.005);
            }
            ExampleSetting::MaxRoughnessStart(adj) => {
                app_settings.max_perceptual_roughness.start =
                    adjust(app_settings.max_perceptual_roughness.start, adj, 0.005);
            }
            ExampleSetting::MaxRoughnessEnd(adj) => {
                app_settings.max_perceptual_roughness.end =
                    adjust(app_settings.max_perceptual_roughness.end, adj, 0.005);
            }
            ExampleSetting::EdgeFadeoutStart(adj) => {
                app_settings.edge_fadeout.start =
                    adjust(app_settings.edge_fadeout.start, adj, 0.001);
            }
            ExampleSetting::EdgeFadeoutEnd(adj) => {
                app_settings.edge_fadeout.end = adjust(app_settings.edge_fadeout.end, adj, 0.001);
            }
        }
    }

    if !any_changes {
        return;
    }

    for (entity, has_background, has_text, _sender) in radio_buttons.iter() {
        if has_background && let Ok(mut background_color) = background_colors.get_mut(entity) {
            *background_color = BackgroundColor(Color::BLACK);
        }
        if has_text {
            update_ui_radio_button_text(entity, &mut writer, false);
        }
    }

    // Update range value text.
    for (parent, marker) in range_value_text.iter() {
        let val = match marker {
            RangeValueText::MinRoughnessStart => app_settings.min_perceptual_roughness.start,
            RangeValueText::MinRoughnessEnd => app_settings.min_perceptual_roughness.end,
            RangeValueText::MaxRoughnessStart => app_settings.max_perceptual_roughness.start,
            RangeValueText::MaxRoughnessEnd => app_settings.max_perceptual_roughness.end,
            RangeValueText::EdgeFadeoutStart => app_settings.edge_fadeout.start,
            RangeValueText::EdgeFadeoutEnd => app_settings.edge_fadeout.end,
        };
        if let Ok(children) = text_children.get(parent) {
            for child in children.iter() {
                if text_query.get(child).is_ok() {
                    *writer.text(child, 0) = format!("{:.2}", val);
                    writer.for_each_color(child, |mut color| {
                        color.0 = Color::BLACK;
                    });
                }
            }
        }
    }
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

fn adjust(val: f32, adj: Adjustment, amount: f32) -> f32 {
    match adj {
        Adjustment::Increase => (val + amount).min(1.0),
        Adjustment::Decrease => (val - amount).max(0.0),
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ssr_on: default(),
            displayed_model: default(),
            displayed_base: default(),
            min_perceptual_roughness: 0.0..0.01,
            max_perceptual_roughness: 0.99..1.0,
            edge_fadeout: 0.0..0.0,
        }
    }
}
