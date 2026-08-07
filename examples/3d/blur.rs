//! Demonstrates blurring selected UI regions over a live 3D scene.
//!
//! The control panel itself is a blur region: use it to switch between the blur
//! algorithms (gaussian, box, dual kawase, bokeh) and tweak their parameters live.

use bevy::{
    camera::Hdr,
    color::palettes::css::{
        AQUAMARINE, DEEP_PINK, GOLD, LIGHT_SKY_BLUE, ORANGE_RED, ROYAL_BLUE, SEA_GREEN,
    },
    core_pipeline::tonemapping::Tonemapping,
    feathers::{
        controls::{FeathersRadio, FeathersSlider},
        dark_theme::create_dark_theme,
        theme::{ThemedText, UiTheme},
        FeathersPlugins,
    },
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_render::{BlurRegion, BlurRegionCamera, BlurSetting, DEFAULT_MAX_BLUR_REGIONS_COUNT},
    ui_widgets::{RadioGroup, SliderPrecision, SliderRange, SliderStep, SliderValue, ValueChange},
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(BlurDemoState::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (spin_meshes, orbit_lights, apply_blur_settings))
        .run();
}

// Blur algorithm selection and parameters

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Algorithm {
    #[default]
    Gaussian,
    BoxBlur,
    DualKawase,
    Bokeh,
}

/// Range and label of one parameter slider for one algorithm.
struct ParamSpec {
    label: &'static str,
    min: f32,
    max: f32,
    step: f32,
}

impl Algorithm {
    fn label(self) -> &'static str {
        match self {
            Algorithm::Gaussian => "Gaussian",
            Algorithm::BoxBlur => "Box blur",
            Algorithm::DualKawase => "Dual Kawase",
            Algorithm::Bokeh => "Bokeh",
        }
    }

    fn param_specs(self) -> [Option<ParamSpec>; 2] {
        match self {
            Algorithm::Gaussian => [
                Some(ParamSpec {
                    label: "Circle of confusion",
                    min: 0.0,
                    max: 400.0,
                    step: 1.0,
                }),
                Some(ParamSpec {
                    label: "Sigma multiplier",
                    min: 0.05,
                    max: 0.5,
                    step: 0.01,
                }),
            ],
            Algorithm::BoxBlur => [
                Some(ParamSpec {
                    label: "Kernel radius",
                    min: 0.0,
                    max: 64.0,
                    step: 1.0,
                }),
                Some(ParamSpec {
                    label: "Sample spacing",
                    min: 0.5,
                    max: 6.0,
                    step: 0.1,
                }),
            ],
            Algorithm::DualKawase => [
                Some(ParamSpec {
                    label: "Mip levels",
                    min: 1.0,
                    max: 6.0,
                    step: 1.0,
                }),
                Some(ParamSpec {
                    label: "Sample offset",
                    min: 0.0,
                    max: 4.0,
                    step: 0.1,
                }),
            ],
            Algorithm::Bokeh => [
                Some(ParamSpec {
                    label: "Aperture radius",
                    min: 1.0,
                    max: 64.0,
                    step: 1.0,
                }),
                None,
            ],
        }
    }
}

/// The active algorithm and the two slider values of every algorithm, preserved
/// across switches.
#[derive(Resource)]
struct BlurDemoState {
    algorithm: Algorithm,
    params: [[f32; 2]; 4],
}

impl Default for BlurDemoState {
    fn default() -> Self {
        BlurDemoState {
            algorithm: Algorithm::Gaussian,
            params: [
                // Gaussian: circle of confusion, sigma multiplier
                [100.0, 0.25],
                // Box blur: kernel radius, sample spacing
                [8.0, 2.0],
                // Dual kawase: mip levels, sample offset
                [3.0, 1.5],
                // Bokeh: aperture radius
                [24.0, 0.0],
            ],
        }
    }
}

impl BlurDemoState {
    fn settings(&self) -> BlurSetting {
        let [primary, secondary] = self.params[self.algorithm as usize];
        match self.algorithm {
            Algorithm::Gaussian => BlurSetting::Gaussian {
                circle_of_confusion: primary,
                sigma_multiplier: secondary,
            },
            Algorithm::BoxBlur => BlurSetting::BoxBlur {
                kernel_radius: primary as u32,
                scale: secondary,
            },
            Algorithm::DualKawase => BlurSetting::DualKawase {
                mip_count: primary as u32,
                offset: secondary,
            },
            Algorithm::Bokeh => BlurSetting::Bokeh {
                radius: primary as u32,
            },
        }
    }
}

/// Marks a radio button as selecting an algorithm.
#[derive(Component, Clone, Copy, Default)]
struct AlgorithmRadio(Algorithm);

/// Marks one of the two parameter sliders. The payload is the parameter slot.
#[derive(Component, Clone, Copy, Default)]
struct ParamSlider(usize);

/// Marks the text label above a parameter slider.
#[derive(Component, Clone, Copy, Default)]
struct ParamLabel(usize);

#[derive(Component)]
struct Spin {
    speed: f32,
}

/// Orbits an entity around `center` at a fixed height and radius.
#[derive(Component)]
struct Orbit {
    center: Vec3,
    radius: f32,
    speed: f32,
    phase: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<BlurDemoState>,
) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Transform::from_xyz(0.0, 5.5, 14.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
        BlurRegionCamera::new(state.settings()),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 80.0,
        ..default()
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, -PI / 4.0, -PI / 3.5)),
    ));

    // Colored point lights orbiting the scene at different radii and speeds.
    let point_lights = [
        (Color::from(ORANGE_RED), 6.0, 0.4, 0.0),
        (Color::from(LIGHT_SKY_BLUE), 9.0, -0.25, 2.1),
        (Color::from(SEA_GREEN), 4.0, 0.6, 4.2),
    ];
    for (color, radius, speed, phase) in point_lights {
        commands.spawn((
            PointLight {
                color,
                intensity: 400_000.0,
                range: 25.0,
                ..default()
            },
            Mesh3d(meshes.add(Sphere::new(0.12).mesh().uv(16, 9))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * 60.0,
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(radius, 3.0, 0.0),
            Orbit {
                center: Vec3::new(0.0, 3.0, -2.0),
                radius,
                speed,
                phase,
            },
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.08, 0.09, 0.11))),
    ));

    // Spinning shapes at a spread of distances from the camera.
    let shapes = [
        (
            meshes.add(Capsule3d::default()),
            materials.add(Color::from(SEA_GREEN)),
            Transform::from_xyz(-2.2, 1.4, 5.5),
            0.5,
        ),
        (
            meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
            materials.add(Color::from(ORANGE_RED)),
            Transform::from_xyz(-4.5, 1.5, 0.0),
            0.7,
        ),
        (
            meshes.add(Sphere::new(1.35).mesh().uv(32, 18)),
            materials.add(Color::from(AQUAMARINE)),
            Transform::from_xyz(0.0, 1.8, -1.5),
            -0.9,
        ),
        (
            meshes.add(Torus::new(0.8, 1.6)),
            materials.add(Color::from(ROYAL_BLUE)),
            Transform::from_xyz(4.5, 2.0, 0.75),
            1.1,
        ),
        (
            meshes.add(Cuboid::new(3.0, 3.0, 3.0)),
            materials.add(Color::from(DEEP_PINK)),
            Transform::from_xyz(-7.0, 2.2, -9.0),
            0.3,
        ),
        (
            meshes.add(Sphere::new(2.0).mesh().uv(32, 18)),
            materials.add(Color::from(GOLD)),
            Transform::from_xyz(7.5, 2.6, -13.0),
            -0.4,
        ),
    ];

    for (mesh, material, transform, speed) in shapes {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            Spin { speed },
        ));
    }

    // Small emissive spheres scattered through the depth of the scene. With the
    // bokeh algorithm these bloom into bright aperture-shaped discs.
    let fairy_lights = [
        (Vec3::new(-1.0, 0.4, 7.0), GOLD),
        (Vec3::new(2.8, 0.6, 4.0), DEEP_PINK),
        (Vec3::new(-5.8, 0.5, 2.5), LIGHT_SKY_BLUE),
        (Vec3::new(1.5, 0.3, 1.0), GOLD),
        (Vec3::new(6.2, 0.7, -2.0), AQUAMARINE),
        (Vec3::new(-3.0, 0.4, -4.5), GOLD),
        (Vec3::new(0.5, 0.6, -7.0), DEEP_PINK),
        (Vec3::new(-8.5, 0.5, -11.0), LIGHT_SKY_BLUE),
        (Vec3::new(4.0, 0.8, -16.0), GOLD),
        (Vec3::new(-2.0, 1.0, -20.0), AQUAMARINE),
    ];
    let fairy_mesh = meshes.add(Sphere::new(0.09).mesh().uv(16, 9));
    for (position, color) in fairy_lights {
        commands.spawn((
            Mesh3d(fairy_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::from(color),
                emissive: Color::from(color).to_linear() * 40.0,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(position),
        ));
    }

    commands.spawn_scene(ui_root(&state));
}

/// The full-screen UI: the control panel plus a large, nearly untinted blur
/// region in the middle of the scene, so the character of each algorithm is easy
/// to compare.
fn ui_root(state: &BlurDemoState) -> impl Scene + use<> {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            control_panel(state),
            (
                BlurRegion
                Node {
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexEnd,
                    padding: UiRect::all(px(14)),
                    border_radius: BorderRadius::all(px(36)),
                }
                BorderColor::all(Color::srgba(0.85, 0.92, 1.0, 0.45))
                Children [(
                    TextFont {
                        font_size: FontSize::Px(15.0),
                    }
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85))
                )]
            ),
        ]
    }
}

/// The blurred control panel: algorithm radio buttons and two parameter sliders.
fn control_panel(state: &BlurDemoState) -> impl Scene + use<> {
    let specs = state.algorithm.param_specs();
    let values = state.params[state.algorithm as usize];

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(10),
            bottom: px(10),
            width: px(340),
            padding: UiRect::axes(px(20), px(18)),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            border_radius: BorderRadius::all(px(28)),
        }
        ZIndex(1)
        BackgroundColor(Color::srgba(0.05, 0.08, 0.11, 0.25))
        BorderColor::all(Color::srgba(0.85, 0.92, 1.0, 0.65))
        Children [
            (
                Text("Blur playground")
                TextFont {
                    font_size: FontSize::Px(26.0),
                }
                TextColor(Color::WHITE)
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                }
                RadioGroup
                on(algorithm_selected)
                Children [
                    // The demo starts on the gaussian algorithm, so it spawns checked.
                    (algorithm_radio(Algorithm::Gaussian) Checked),
                    algorithm_radio(Algorithm::BoxBlur),
                    algorithm_radio(Algorithm::DualKawase),
                    algorithm_radio(Algorithm::Bokeh),
                ]
            ),
            param_label(0, &specs[0]),
            param_slider(0, values[0], &specs[0]),
            param_label(1, &specs[1]),
            param_slider(1, values[1], &specs[1]),
        ]
    }
}

fn algorithm_radio(algorithm: Algorithm) -> impl Scene {
    let label = algorithm.label();
    bsn! {
        @FeathersRadio {
            @caption: bsn! { Text(label) ThemedText }
        }
        AlgorithmRadio(algorithm)
    }
}

fn param_label(slot: usize, spec: &Option<ParamSpec>) -> impl Scene + use<> {
    let label = spec.as_ref().map_or("(unused)", |spec| spec.label);
    bsn! {
        ParamLabel(slot)
        Text(label)
        TextFont {
            font_size: FontSize::Px(14.0),
        }
        TextColor(Color::srgba(0.93, 0.97, 1.0, 0.92))
    }
}

fn param_slider(slot: usize, value: f32, spec: &Option<ParamSpec>) -> impl Scene + use<> {
    let (min, max, step) = spec
        .as_ref()
        .map_or((0.0, 1.0, 0.1), |spec| (spec.min, spec.max, spec.step));

    bsn! {
        @FeathersSlider {
            @value: value,
            @min: min,
            @max: max,
        }
        SliderStep(step)
        SliderPrecision({ step_precision(step).0 })
        ParamSlider(slot)
        on(move |change: On<ValueChange<f32>>,
                 mut state: ResMut<BlurDemoState>,
                 mut commands: Commands| {
            commands
                .entity(change.source)
                .insert(SliderValue(change.value));
            let algorithm = state.algorithm;
            state.params[algorithm as usize][slot] = change.value;
        })
    }
}

/// The number of decimals shown on a slider: whole numbers for integer-stepped
/// parameters, two decimals otherwise.
fn step_precision(step: f32) -> SliderPrecision {
    SliderPrecision(if step >= 1.0 { 0 } else { 2 })
}

/// Handles a radio button selection: updates the checked states and the demo state.
fn algorithm_selected(
    change: On<ValueChange<Entity>>,
    radios: Query<(Entity, &AlgorithmRadio)>,
    mut state: ResMut<BlurDemoState>,
    mut commands: Commands,
) {
    for (entity, algorithm_radio) in &radios {
        if entity == change.value {
            commands.entity(entity).insert(Checked);
            state.algorithm = algorithm_radio.0;
        } else {
            commands.entity(entity).remove::<Checked>();
        }
    }
}

/// Pushes the demo state into the camera, and refreshes the slider labels and
/// ranges when the algorithm changes.
fn apply_blur_settings(
    state: Res<BlurDemoState>,
    mut previous_algorithm: Local<Option<Algorithm>>,
    mut blur_cameras: Query<&mut BlurRegionCamera<{ DEFAULT_MAX_BLUR_REGIONS_COUNT }>>,
    sliders: Query<(Entity, &ParamSlider)>,
    mut labels: Query<(&ParamLabel, &mut Text)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }

    for mut camera in &mut blur_cameras {
        camera.settings = state.settings();
    }

    if *previous_algorithm == Some(state.algorithm) {
        return;
    }
    *previous_algorithm = Some(state.algorithm);

    // Re-target the sliders and labels to the new algorithm's parameters.
    let specs = state.algorithm.param_specs();
    let values = state.params[state.algorithm as usize];

    for (entity, param_slider) in &sliders {
        let slot = param_slider.0;
        match &specs[slot] {
            Some(spec) => {
                commands.entity(entity).insert((
                    SliderRange::new(spec.min, spec.max),
                    SliderValue(values[slot].clamp(spec.min, spec.max)),
                    SliderStep(spec.step),
                    step_precision(spec.step),
                ));
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            None => {
                commands
                    .entity(entity)
                    .insert((InteractionDisabled, SliderValue(0.0)));
            }
        }
    }

    for (param_label, mut text) in &mut labels {
        text.0 = specs[param_label.0]
            .as_ref()
            .map_or("(unused)", |spec| spec.label)
            .to_string();
    }
}

fn spin_meshes(mut query: Query<(&Spin, &mut Transform)>, time: Res<Time>) {
    for (spin, mut transform) in &mut query {
        transform.rotate_y(time.delta_secs() * spin.speed);
        transform.rotate_x(time.delta_secs() * spin.speed * 0.35);
    }
}

fn orbit_lights(mut query: Query<(&Orbit, &mut Transform)>, time: Res<Time>) {
    for (orbit, mut transform) in &mut query {
        let angle = time.elapsed_secs() * orbit.speed + orbit.phase;
        let offset = Vec2::from_angle(angle) * orbit.radius;
        transform.translation = orbit.center + Vec3::new(offset.x, 0.0, offset.y);
    }
}
