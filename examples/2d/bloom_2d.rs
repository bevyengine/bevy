//! Illustrates bloom post-processing in 2d.

use bevy::{
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    feathers::{
        constants::icons,
        containers::*,
        controls::{
            ButtonVariant, FeathersCheckbox, FeathersListRow, FeathersListView,
            FeathersNumberInput, FeathersSlider, FeathersToolButton, NumberInputPrecision,
            NumberInputValue,
        },
        display::{icon, label},
        theme::UiTheme,
        FeathersPlugins,
    },
    math::Rot2,
    post_process::bloom::{Bloom, BloomCompositeMode},
    prelude::*,
    ui::{Selected, UiTransform},
    ui_widgets::{
        checkbox_self_update, listbox_update_selection, radio_self_update, Activate,
        SliderPrecision, SliderStep, SliderValue, ValueChange,
    },
};
use std::f32::consts::FRAC_PI_2;

use checkbox::feathers_option_checkbox;
use radio::{feathers_option_buttons, RadioButtonOptionValue};

#[path = "../helpers/checkbox.rs"]
mod checkbox;

/// NOTE: Requires `pub` if `main_ui_node_scene()` is not used.
#[path = "../helpers/radio.rs"]
pub mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_observer(handle_value_change_checkbox)
        .add_observer(update_radio_button)
        .add_observer(checkbox_self_update)
        .add_observer(radio_self_update)
        .run();
}

#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum CheckboxInput {
    #[default]
    BloomOff,
}

#[derive(Clone, Copy, Component, Default, PartialEq, Debug, FromTemplate)]
enum SliderInput {
    #[default]
    Intensity,
    LowFrequencyBoost,
    LowFrequencyBoostCurvature,
    HighPassFrequency,
    ThresholdSoftness,
    HorizontalScale,
}

#[derive(Clone, Copy, Component, Default)]
struct TonemappingOption(Tonemapping);

#[derive(Clone, Copy, Component, Default)]
struct BloomSettingsPane;

#[derive(Clone, Copy, Component, Default)]
struct PaneBody;

#[derive(Clone, Copy, Component, Default)]
struct PaneToggleIcon;

#[derive(Resource)]
struct BloomPaneParent(Entity);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::TonyMcMapface, // 1. Using a tonemapper that desaturates to white is recommended
        Bloom::default(),           // 2. Enable bloom for the camera
        DebandDither::Enabled,      // Optional: bloom causes gradients which cause banding
    ));

    // Sprite
    commands.spawn(Sprite {
        image: asset_server.load("branding/bevy_bird_dark.png"),
        color: Color::srgb(5.0, 5.0, 5.0), // 3. Put something bright in a dark environment to see the effect
        custom_size: Some(Vec2::splat(160.0)),
        ..default()
    });

    // Circle mesh
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(100.))),
        // 3. Put something bright in a dark environment to see the effect
        MeshMaterial2d(materials.add(Color::srgb(7.5, 0.0, 7.5))),
        Transform::from_translation(Vec3::new(-200., 0., 0.)),
    ));

    // Hexagon mesh
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(100., 6))),
        // 3. Put something bright in a dark environment to see the effect
        MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
        Transform::from_translation(Vec3::new(200., 0., 0.)),
    ));

    // UI
    let root = commands
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: UiRect::all(px(8)),
            row_gap: px(8),
            width: percent(30),
            min_width: px(300),
            ..default()
        })
        .id();

    commands.spawn_scene(bloom_off_checkbox(root));
    commands.spawn_scene(bloom_settings_pane(root));

    commands.insert_resource(BloomPaneParent(root));
}

// ------------------------------------------------------------------------------------------------
fn bloom_off_checkbox(parent: Entity) -> impl Scene {
    bsn! {
        ChildOf(parent) feathers_option_checkbox("Bloom OFF", Some(CheckboxInput::BloomOff))
    }
}

fn bloom_settings_pane(parent: Entity) -> impl Scene {
    bsn! {
        pane() ChildOf(parent) BloomSettingsPane Children [
            pane_header() Children [
                label("Options"),
                flex_spacer(),
                @FeathersToolButton {
                    @variant: ButtonVariant::Plain,
                } Children [
                    icon(icons::CHEVRON_DOWN) PaneToggleIcon
                ]
                on(toggle_pane_body)
            ],
            pane_body() PaneBody Children [
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children[label("Intensity")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 1.,
                                @min: 0.,
                            }
                            Node { flex_grow: 1. }
                            SliderValue(0.15)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::Intensity
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("Low Frequency Boost")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 1.,
                                @min: 0.,
                            }
                            SliderValue(0.70)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::LowFrequencyBoost
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("Low Frequency Boost Curvature")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 1.,
                                @min: 0.,
                            }
                            SliderValue(0.95)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::LowFrequencyBoostCurvature
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("High Pass Frequency")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 1.,
                                @min: 0.,
                            }
                            SliderValue(1.)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::HighPassFrequency
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("Threshold")]
                        ),
                        (
                            @FeathersNumberInput
                            template_value(NumberInputValue::F32(0.))
                            NumberInputPrecision(2)
                            Node { flex_grow: 1. }
                            on(input_value_change)
                        ),
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("ThresholdSoftness")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 1.,
                                @min: 0.,
                            }
                            SliderValue(0.)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::ThresholdSoftness
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("HorizontalScale")]
                        ),
                        (
                            @FeathersSlider{
                                @max: 16.,
                                @min: 0.,
                            }
                            SliderValue(1.)
                            SliderPrecision(2)
                            SliderStep(0.1)
                            SliderInput::HorizontalScale
                            on(slider_update)
                        )
                    ]
                ),
                (
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4)
                    }
                    Children [
                        (
                            Node {
                                width: px(150),
                                flex_shrink: 0.,
                            }
                            Children [label("Composite Mode")]
                        ),
                        (
                            feathers_option_buttons("",
                                &[
                                    (BloomCompositeMode::EnergyConserving, "Energy Conserving"),
                                    (BloomCompositeMode::Additive, "Additive"),
                                ], 0)
                            Node { flex_grow: 1., flex_wrap: FlexWrap::Wrap, }
                        ),
                    ]
                ),
                (
                    subpane() Children[
                        subpane_header() Children [
                            label("Tonemapping")
                        ],
                        subpane_body() Children [
                            @FeathersListView {
                                @rows: { bsn_list![
                                    (
                                        @FeathersListRow
                                        Children[label("None")]
                                        TonemappingOption(Tonemapping::None)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("AcesFitted")]
                                        TonemappingOption(Tonemapping::AcesFitted)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("AgX")]
                                        TonemappingOption(Tonemapping::AgX)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("BlenderFilmic")]
                                        TonemappingOption(Tonemapping::BlenderFilmic)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("Reinhard")]
                                        TonemappingOption(Tonemapping::Reinhard)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("ReinhardLuminance")]
                                        TonemappingOption(Tonemapping::ReinhardLuminance)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("SomewhatBoringDisplayTransform")]
                                        TonemappingOption(Tonemapping::SomewhatBoringDisplayTransform)
                                    ),
                                    (
                                        @FeathersListRow Selected
                                        Children[label("TonyMcMapface")]
                                        TonemappingOption(Tonemapping::TonyMcMapface)
                                    ),
                                    (
                                        @FeathersListRow
                                        Children[label("KhronosPbrNeutral")]
                                        TonemappingOption(Tonemapping::KhronosPbrNeutral)
                                    ),
                                ]}
                            }
                            Node {
                                max_height: px(100),
                            }
                            on(listbox_update_selection)
                            on(apply_tonemapping_change)
                        ],
                    ]
                ),
            ]
        ]
    }
}

fn handle_value_change_checkbox(
    event: On<ValueChange<bool>>,
    mut commands: Commands,
    camera: Single<Entity, With<Camera>>,
    checkbox_input_query: Query<&CheckboxInput, With<FeathersCheckbox>>,
    pane_query: Query<Entity, With<BloomSettingsPane>>,
    pane_parent: Res<BloomPaneParent>,
) {
    let camera_entity = camera.into_inner();
    if let Ok(checkbox_input) = checkbox_input_query.get(event.source) {
        match checkbox_input {
            CheckboxInput::BloomOff => {
                if event.value {
                    commands.entity(camera_entity).remove::<Bloom>();
                    if let Ok(pane_entity) = pane_query.single() {
                        commands.entity(pane_entity).despawn();
                    }
                } else {
                    commands
                        .entity(camera_entity)
                        .insert((Bloom::default(), Tonemapping::TonyMcMapface));
                    commands.spawn_scene(bloom_settings_pane(pane_parent.0));
                }
            }
        }
    }
}

fn toggle_pane_body(
    _event: On<Activate>,
    mut commands: Commands,
    mut pane_body_query: Query<&mut Node, With<PaneBody>>,
    icon_query: Query<Entity, With<PaneToggleIcon>>,
) {
    let Ok(mut node) = pane_body_query.single_mut() else {
        return;
    };

    let will_open = node.display == Display::None;
    node.display = if will_open {
        Display::Flex
    } else {
        Display::None
    };

    if let Ok(icon_entity) = icon_query.single() {
        let rotation = if will_open {
            Rot2::IDENTITY
        } else {
            Rot2::radians(FRAC_PI_2)
        };
        commands
            .entity(icon_entity)
            .insert(UiTransform::from_rotation(rotation));
    }
}

fn slider_update(
    value_change: On<ValueChange<f32>>,
    camera: Single<Option<&mut Bloom>, With<Camera>>,
    mut commands: Commands,
    slider_query: Query<&SliderInput>,
) {
    commands
        .entity(value_change.source)
        .insert(SliderValue(value_change.value));

    let bloom = camera.into_inner();
    if let Some(mut bloom) = bloom
        && let Ok(slider_input) = slider_query.get(value_change.source)
    {
        match slider_input {
            SliderInput::Intensity => {
                bloom.intensity = value_change.value;
            }
            SliderInput::LowFrequencyBoost => {
                bloom.low_frequency_boost = value_change.value;
            }
            SliderInput::LowFrequencyBoostCurvature => {
                bloom.low_frequency_boost_curvature = value_change.value;
            }
            SliderInput::HighPassFrequency => {
                bloom.high_pass_frequency = value_change.value;
            }
            SliderInput::ThresholdSoftness => {
                bloom.prefilter.threshold_softness = value_change.value;
            }
            SliderInput::HorizontalScale => {
                bloom.scale.x = value_change.value;
            }
        }
    }
}

fn apply_tonemapping_change(
    event: On<ValueChange<Entity>>,
    tonemapping_query: Query<&TonemappingOption>,
    camera: Single<Entity, With<Camera>>,
    mut commands: Commands,
) {
    let camera_entity = camera.into_inner();
    if let Ok(option) = tonemapping_query.get(event.value) {
        commands.entity(camera_entity).insert(option.0);
    }
}

fn update_radio_button(
    event: On<ValueChange<Entity>>,
    bloom_query: Query<&RadioButtonOptionValue<BloomCompositeMode>>,
    camera: Single<Option<&mut Bloom>, With<Camera>>,
) {
    let bloom = camera.into_inner();
    if let Ok(RadioButtonOptionValue(option)) = bloom_query.get(event.value)
        && let Some(mut bloom) = bloom
    {
        bloom.composite_mode = *option;
    }
}

fn input_value_change(
    value_change: On<ValueChange<f32>>,
    camera: Single<Option<&mut Bloom>, With<Camera>>,
    mut commands: Commands,
) {
    commands
        .entity(value_change.source)
        .insert(NumberInputValue::F32(value_change.value));

    let bloom = camera.into_inner();
    if let Some(mut bloom) = bloom {
        bloom.prefilter.threshold = value_change.value;
    }
}
