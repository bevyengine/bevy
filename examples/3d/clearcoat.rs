//! Demonstrates the clearcoat PBR feature.
//!
//! Clearcoat is a separate material layer that represents a thin translucent
//! layer over a material. Examples include (from the Filament spec [1]) car paint,
//! soda cans, and lacquered wood.
//!
//! In glTF, clearcoat is supported via the `KHR_materials_clearcoat` [2]
//! extension. This extension is well supported by tools; in particular,
//! Blender's glTF exporter maps the clearcoat feature of its Principled BSDF
//! node to this extension, allowing it to appear in Bevy.
//!
//! This Bevy example is inspired by the corresponding three.js example [3].
//!
//! [1]: https://google.github.io/filament/Filament.md.html#materialsystem/clearcoatmodel
//!
//! [2]: https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md
//!
//! [3]: https://threejs.org/examples/webgl_materials_physical_clearcoat.html

use std::f32::consts::PI;

use bevy::{
    camera::Hdr,
    color::palettes::css::{BLUE, GOLD, WHITE},
    core_pipeline::tonemapping::Tonemapping::AcesFitted, 
    ecs::{system::{SystemParam}, VariantDefaults},
    feathers::{
        containers::*,
        controls::*,
        dark_theme::create_dark_theme,
        display::label_small,
        theme::{ThemedText, UiTheme},
        FeathersPlugins,
    },
    image::ImageLoaderSettings,
    input_focus::tab_navigation::TabGroup,
    light::Skybox,
    math::vec3,
    prelude::*,
    ui::Checked,
    ui_widgets::{
        checkbox_self_update, radio_self_update, slider_self_update, RadioGroup, SliderPrecision,
        SliderStep, SliderValue, ValueChange,
    },
};


/// The size of each sphere.
const SPHERE_SCALE: f32 = 0.9;

/// Which type of light we're using: a point light or a directional light.
#[derive(Clone, Copy, PartialEq, Resource, Default)]
enum LightMode {
    #[default]
    Point,
    Directional,
}

/// Tags the example spheres.
#[derive(Component)]
struct ExampleSphere;

/// Entry point.
pub fn main() {
    App::new()
        .init_resource::<LightMode>()
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(TweakableKnobState {
            illuminance: 10000.0,
            intensity: 1000.0,
            rotation_speed: 0.8,
        })
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .add_systems(Startup, (scene.spawn(), hide_control_pane).chain())
        .add_systems(Startup, setup)
        .add_systems(Startup, init_rotation_speed)
        .add_systems(Update, animate_light)
        .add_systems(Update, animate_spheres)
        .run();
}

/// Initializes the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    states: Res<TweakableKnobState>,
    asset_server: Res<AssetServer>,
) {
    let sphere = create_sphere_mesh(&mut meshes);
    spawn_car_paint_sphere(&mut commands, &mut materials, &asset_server, &sphere);
    spawn_coated_glass_bubble_sphere(&mut commands, &mut materials, &sphere);
    spawn_golf_ball(&mut commands, &asset_server);
    spawn_scratched_gold_ball(&mut commands, &mut materials, &asset_server, &sphere);

    spawn_light(&mut commands, states);
    spawn_camera(&mut commands, &asset_server);
}

/// Generates a sphere.
fn create_sphere_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    // We're going to use normal maps, so make sure we've generated tangents, or
    // else the normal maps won't show up.

    let mut sphere_mesh = Sphere::new(1.0).mesh().build();
    sphere_mesh
        .generate_tangents()
        .expect("Failed to generate tangents");
    meshes.add(sphere_mesh)
}

/// Spawn a regular object with a clearcoat layer. This looks like car paint.
fn spawn_car_paint_sphere(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(
                materials.add(StandardMaterial {
                    clearcoat: 1.0,
                    clearcoat_perceptual_roughness: 0.1,
                    normal_map_texture: Some(
                        asset_server
                            .load_builder()
                            .with_settings(|settings: &mut ImageLoaderSettings| {
                                settings.is_srgb = false;
                            })
                            .load("textures/BlueNoise-Normal.png"),
                    ),
                    metallic: 0.9,
                    perceptual_roughness: 0.5,
                    base_color: BLUE.into(),
                    ..default()
                }),
            ),
            Transform::from_xyz(-1.0, 1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
        ))
        .insert(ExampleSphere);
}

/// Spawn a semitransparent object with a clearcoat layer.
fn spawn_coated_glass_bubble_sphere(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                clearcoat: 1.0,
                clearcoat_perceptual_roughness: 0.1,
                metallic: 0.5,
                perceptual_roughness: 0.1,
                base_color: Color::srgba(0.9, 0.9, 0.9, 0.3),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(-1.0, -1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
        ))
        .insert(ExampleSphere);
}

/// Spawns an object with both a clearcoat normal map (a scratched varnish) and
/// a main layer normal map (the golf ball pattern).
///
/// This object is in glTF format, using the `KHR_materials_clearcoat`
/// extension.
fn spawn_golf_ball(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/GolfBall/GolfBall.glb")),
        ),
        Transform::from_xyz(1.0, 1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
        ExampleSphere,
    ));
}

/// Spawns an object with only a clearcoat normal map (a scratch pattern) and no
/// main layer normal map.
fn spawn_scratched_gold_ball(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(
                materials.add(StandardMaterial {
                    clearcoat: 1.0,
                    clearcoat_perceptual_roughness: 0.3,
                    clearcoat_normal_texture: Some(
                        asset_server
                            .load_builder()
                            .with_settings(|settings: &mut ImageLoaderSettings| {
                                settings.is_srgb = false;
                            })
                            .load("textures/ScratchedGold-Normal.png"),
                    ),
                    metallic: 0.9,
                    perceptual_roughness: 0.1,
                    base_color: GOLD.into(),
                    ..default()
                }),
            ),
            Transform::from_xyz(1.0, -1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
        ))
        .insert(ExampleSphere);
}

/// Spawns a light.
fn spawn_light(commands: &mut Commands, states: Res<TweakableKnobState>) {
    commands.spawn(create_directional_light(states.intensity));
}

/// Spawns a camera with associated skybox and environment map.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn((
            Camera3d::default(),
            Hdr,
            Projection::Perspective(PerspectiveProjection {
                fov: 27.0 / 180.0 * PI,
                ..default()
            }),
            Transform::from_xyz(0.0, 0.0, 10.0),
            AcesFitted,
        ))
        .insert(Skybox {
            brightness: 5000.0,
            image: Some(asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2")),
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        });
}

/// Moves the light around.
fn animate_light(
    mut lights: Query<&mut Transform, Or<(With<PointLight>, With<DirectionalLight>)>>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for mut transform in lights.iter_mut() {
        transform.translation = vec3(
            ops::sin(now * 1.4),
            ops::cos(now * 1.0),
            ops::cos(now * 0.6),
        ) * vec3(3.0, 4.0, 3.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Rotates the spheres.
fn animate_spheres(
    mut spheres: Query<&mut Transform, With<ExampleSphere>>,
    time: Res<Time>,
    ui: SharedUiState,
) {
    let now = time.elapsed_secs();
    for mut transform in spheres.iter_mut() {
        transform.rotation = Quat::from_rotation_y(ui.knobs.rotation_speed * now);
    }
}

/// Creates or recreates the moving point light.
fn create_point_light(intensity: f32) -> PointLight {
    PointLight {
        color: WHITE.into(),
        intensity,
        ..default()
    }
}

/// Creates or recreates the moving directional light.
fn create_directional_light(illuminance: f32) -> DirectionalLight {
    DirectionalLight {
        color: WHITE.into(),
        illuminance,
        ..default()
    }
}

#[derive(SystemParam)]
struct SharedUiState<'w, 's> {
    light_mode: ResMut<'w, LightMode>,
    light_query: Query<'w, 's, Entity, Or<(With<PointLight>, With<DirectionalLight>)>>,
    commands: Commands<'w, 's>,
    knobs: ResMut<'w, TweakableKnobState>,
    speed_query: Query<'w, 's, Entity, With<RotationSpeedScalarField>>,
}

#[derive(Resource)]
struct TweakableKnobState {
    illuminance: f32,
    intensity: f32,
    rotation_speed: f32,
}

#[derive(Component, Clone, Copy, Default)]
struct RotationSpeedScalarField;

#[derive(Component, Clone, Copy, Default, VariantDefaults)]
enum RadioLightMode {
    #[default]
    Point,
    Directional,
}

#[derive(Component, Clone, Copy, Default)]
struct UiControlsPaneBody;

fn scene() -> impl SceneList {
    bsn_list![ui()]
}

fn ui() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
        }
        Pickable::IGNORE
        TabGroup
        Children[
            control_pane(),
        ]
    }
}

/// Sets the illuminance of the directional light source to the slider's current value.
fn update_illuminance(new_value: f32, mut ui: SharedUiState) {
    ui.knobs.illuminance = new_value;
    for light in ui.light_query.iter_mut() {
        match *(ui.light_mode) {
            LightMode::Point => {} // PointLight has no illuminance field
            LightMode::Directional => {
                ui.commands
                    .entity(light)
                    .insert(create_directional_light(ui.knobs.illuminance));
            }
        }
    }
}

/// Sets the intensity of the point light source to the slider's current value.
fn update_intensity(new_value: f32, mut ui: SharedUiState) {
    ui.knobs.intensity = new_value;
    for light in ui.light_query.iter_mut() {
        match *(ui.light_mode) {
            LightMode::Point => {
                ui.commands
                    .entity(light)
                    .insert(create_point_light(ui.knobs.intensity));
            }
            LightMode::Directional => {} // DirectionalLight has no intensity field
        }
    }
}

/// Sets the rotation speed of the spheres to the scalar input field's current value.
fn update_rotation_speed(new_value: f32, mut ui: SharedUiState) {
    ui.knobs.rotation_speed = new_value;
    for scalar_input_ent in ui.speed_query.iter() {
        ui.commands
            .entity(scalar_input_ent)
            .insert(NumberInputValue::F32(ui.knobs.rotation_speed));
    }
}

/// Toggles point light.
fn toggle_point_light(mut ui: SharedUiState) {
    for light in ui.light_query.iter_mut() {
        *(ui.light_mode) = LightMode::Point;
        ui.commands
            .entity(light)
            .remove::<DirectionalLight>()
            .insert(create_point_light(ui.knobs.intensity));
    }
}

/// Toggles directional light.
fn toggle_directional_light(mut ui: SharedUiState) {
    for light in ui.light_query.iter_mut() {
        *(ui.light_mode) = LightMode::Directional;
        ui.commands
            .entity(light)
            .remove::<PointLight>()
            .insert(create_directional_light(ui.knobs.illuminance));
    }
}

/// Sets the light type to the currently checked radio button.
fn hit_the_lights(checked: RadioLightMode, mut ui: SharedUiState) {
    match checked {
        RadioLightMode::Directional => {
            *(ui.light_mode) = LightMode::Directional;
            toggle_directional_light(ui);
        }
        RadioLightMode::Point => {
            *(ui.light_mode) = LightMode::Point;
            toggle_point_light(ui);
        }
    }
}

/// Collapses all active control panes.  
fn collapse_control_panes(
    change: On<ValueChange<bool>>,
    mut panes: Query<&mut Node, With<UiControlsPaneBody>>,
) {
    for mut node in &mut panes {
        node.display = if change.value {
            Display::Flex
        } else {
            Display::None
        }
    }
}

/// A set of widget controls for tweaking light property knobs, grouped into a pane.
fn control_pane() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            width: percent(30),
            min_width: px(200),
            flex_direction: FlexDirection::Column,
        }
        pane() Children [
            pane_header() Children [
                (@FeathersDisclosureToggle
                    on(checkbox_self_update)
                    on(|change: On<ValueChange<bool>>,
                        panes: Query<&mut Node,
                        With<UiControlsPaneBody>> | {
                            collapse_control_panes(change, panes);
                    })
                ),
                (Text("UI Controls Pane") ThemedText),
            ],
            (
                UiControlsPaneBody
                pane_body() Children [
                    label_small("Illuminance"),
                    (
                        @FeathersSlider {
                            @max: 10000.0,
                        }
                        SliderStep(100.)
                        SliderPrecision(2)
                        SliderValue(10000.0)
                        on(slider_self_update)
                        on(|change: On<ValueChange<f32>>,ui: SharedUiState| {
                            update_illuminance(change.value, ui);
                        })
                    ),
                    label_small("Intensity"),
                    (
                        @FeathersSlider {
                            @max: 100000.0,
                        }
                        SliderStep(1000.)
                        SliderPrecision(2)
                        SliderValue(1000.0)
                        on(slider_self_update)
                        on(|change: On<ValueChange<f32>>, ui: SharedUiState| {
                            update_intensity(change.value, ui);
                        })
                    ),
                    label_small("Rotation Speed"),
                    (
                        @FeathersNumberInput
                        RotationSpeedScalarField
                        Node {
                            flex_grow: 1.0,
                            max_width: px(100),
                        }
                        on(|change: On<ValueChange<f32>>, ui: SharedUiState| {
                            if change.is_final {
                                update_rotation_speed(change.value, ui);
                            }
                        })
                    ),
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    RadioGroup
                    on(radio_self_update)
                    on(|change: On<ValueChange<Entity>>,
                        ui: SharedUiState,
                        q_light: Query<&RadioLightMode>| {
                            if let Ok(&checked) = q_light.get(change.value) {
                                hit_the_lights(checked, ui);
                            }
                    })
                    Children [
                        (@FeathersRadio { @caption: bsn!{ Text("Point Light") ThemedText } } Checked RadioLightMode::Point),
                        (@FeathersRadio { @caption: bsn!{ Text("Directional Light") ThemedText } }   RadioLightMode::Directional),
                    ]
                ]
            )
        ]
    }
}

fn init_rotation_speed(
    mut commands: Commands,
    states: Res<TweakableKnobState>,
    q_scalar_input: Query<Entity, With<RotationSpeedScalarField>>,
) {
    for scalar_input_ent in q_scalar_input.iter() {
        commands
            .entity(scalar_input_ent)
            .insert(NumberInputValue::F32(states.rotation_speed));
    }
}

fn hide_control_pane(mut panes: Query<&mut Node, With<UiControlsPaneBody>>) {
    for mut node in &mut panes {
        node.display = Display::None;
    }
}
