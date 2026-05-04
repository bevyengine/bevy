//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    ecs::VariantDefaults,
    feathers::{
        constants::{fonts, icons},
        containers::*,
        controls::*,
        cursor::{EntityCursor, OverrideCursor},
        dark_theme::create_dark_theme,
        display::{icon, label, label_dim, label_small},
        font_styles::InheritableFont,
        palette,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::{tab_navigation::TabGroup, AutoFocus, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled},
    ui_widgets::{
        checkbox_self_update, radio_self_update, slider_self_update, Activate, ActivateOnPress,
        RadioGroup, SliderPrecision, SliderStep, SliderValue, ValueChange,
    },
    window::SystemCursorIcon,
};

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
    scalar_prop: f32,
    vec3_prop: Vec3,
}

#[derive(Component, Clone, Copy, PartialEq, FromTemplate)]
enum SwatchType {
    #[default]
    Rgb,
    Hsl,
}

#[derive(Component, Clone, Copy, Default)]
struct HexColorInput;

#[derive(Component, Clone, Copy, Default)]
struct DemoDisabledButton;

#[derive(Component, Clone, Copy, Default)]
struct DemoScalarField;

#[derive(Component, Clone, Copy, Default, VariantDefaults)]
enum DemoVec3Field {
    #[default]
    X,
    Y,
    Z,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(DemoWidgetStates {
            rgb_color: palettes::tailwind::EMERALD_800.with_alpha(0.7),
            hsl_color: palettes::tailwind::AMBER_800.into(),
            scalar_prop: 7.0,
            vec3_prop: Vec3::new(10.1, 7.124, 100.0),
        })
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, update_colors)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, demo_root()]
}

fn demo_root() -> impl Scene {
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
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[
            :demo_column_1,
            :demo_column_2,
        ]
    }
}

fn demo_column_1() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Normal") ThemedText }}
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Normal button clicked!");
                        })
                        AutoFocus
                    ),
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Disabled") ThemedText }},
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        InteractionDisabled
                        DemoDisabledButton
                        on(|_activate: On<Activate>| {
                            info!("Disabled button clicked!");
                        })
                    ),
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Primary") ThemedText }},
                            @variant: ButtonVariant::Primary,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Primary button clicked!");
                        })
                    ),
                    (
                        :FeathersMenu
                        Children [
                            (
                                :FeathersMenuButton {
                                    @caption: {bsn! { Text("Menu") ThemedText }}
                                }
                                Node {
                                    flex_grow: 1.0,
                                }
                            ),
                            (
                                :FeathersMenuPopup
                                Children [
                                    (
                                        :FeathersMenuItem {
                                            @caption: {bsn! { Text("MenuItem 1") ThemedText }}
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 1 clicked!");
                                        })
                                    ),
                                    (
                                        :FeathersMenuItem {
                                            @caption: {bsn! { Text("MenuItem 2") ThemedText }}
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 2 clicked!");
                                        })
                                    ),
                                    :FeathersMenuDivider,
                                    (
                                        :FeathersMenuItem {
                                            @caption: {bsn! { Text("MenuItem 3") ThemedText }}
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 3 clicked!");
                                        })
                                    )
                                ]
                            )
                        ]
                    )
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(1),
                }
                Children [
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Left") ThemedText }},
                            @corners: RoundedCorners::Left,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Left button clicked!");
                        })
                    ),
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Center") ThemedText }},
                            @corners: RoundedCorners::None,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Center button clicked!");
                        })
                    ),
                    (
                        :FeathersButton {
                            @caption: {bsn! { Text("Right") ThemedText }},
                            @variant: ButtonVariant::Primary,
                            @corners: RoundedCorners::Right,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Right button clicked!");
                        })
                    ),
                ]
            ),
            (
                :FeathersButton
                on(|_activate: On<Activate>, mut ovr: ResMut<OverrideCursor>| {
                    ovr.0 = if ovr.0.is_some() {
                        None
                    } else {
                        Some(EntityCursor::System(SystemCursorIcon::Wait))
                    };
                    info!("Override cursor button clicked!");
                })
                Children [ (Text("Toggle override") ThemedText) ]
            ),
            (
                :FeathersCheckbox {
                    @caption: {bsn! { Text("Checkbox") ThemedText }}
                }
                Checked
                on(
                    |change: On<ValueChange<bool>>,
                        query: Query<Entity, With<DemoDisabledButton>>,
                        mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let mut button = commands.entity(query.single().unwrap());
                        if change.value {
                            button.insert(InteractionDisabled);
                        } else {
                            button.remove::<InteractionDisabled>();
                        }
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                :FeathersCheckbox {
                    @caption: {bsn! { Text("Fast Click Checkbox") ThemedText }}
                }
                ActivateOnPress
                on(
                    |change: On<ValueChange<bool>>,
                     mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                :FeathersCheckbox {
                    @caption: {bsn! { Text("Disabled") ThemedText }},
                }
                InteractionDisabled
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                :FeathersCheckbox {
                    @caption: {bsn! { Text("Checked+Disabled") ThemedText }}
                }
                InteractionDisabled
                Checked
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                        }
                        RadioGroup
                        on(radio_self_update)
                        Children [
                            (
                                :FeathersRadio {
                                    @caption: {bsn! { Text("One") ThemedText }}
                                }
                                Checked
                            ),
                            :FeathersRadio {
                                @caption: {bsn! { Text("Two") ThemedText }}
                            },
                            (
                                :FeathersRadio {
                                    @caption: {bsn! { Text("Fast Click") ThemedText }}
                                }
                                ActivateOnPress
                            ),
                            (
                                :FeathersRadio {
                                    @caption: {bsn! { Text("Disabled") ThemedText }}
                                }
                                InteractionDisabled
                            ),
                        ]
                    )
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (:FeathersToggleSwitch on(checkbox_self_update)),
                    (:FeathersToggleSwitch ActivateOnPress on(checkbox_self_update)),
                    (:FeathersToggleSwitch InteractionDisabled on(checkbox_self_update)),
                    (:FeathersToggleSwitch InteractionDisabled Checked on(checkbox_self_update)),
                    (:FeathersDisclosureToggle on(checkbox_self_update)),
                ]
            ),
            (
                :FeathersSlider {
                    @max: 100.0,
                    @value: 20.0,
                }
                SliderStep(10.)
                SliderPrecision(2)
                on(slider_self_update)
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(4),
                }
                Children [
                    :label("Srgba"),
                    // Spacer
                    :flex_spacer,
                    // Text input
                    (
                        :FeathersTextInputContainer
                        Node {
                            flex_grow: 0.
                            padding: { px(4).left() },
                        }
                        Children [
                            (
                                :FeathersTextInput {
                                    @visible_width: 10f32,
                                    @max_characters: 9usize,
                                }
                                InheritableFont {
                                    font: fonts::MONO
                                }
                                HexColorInput
                                on(handle_hex_color_change)
                            )
                        ]
                    )
                    (:FeathersColorSwatch SwatchType::Rgb),
                ]
            ),
            (
                :FeathersColorPlane::RedBlue
                on(|change: On<ValueChange<Vec2>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value.x;
                    color.rgb_color.blue = change.value.y;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Red
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Green
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.green = change.value;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Blue
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.blue = change.value;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Alpha
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.alpha = change.value;
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                }
                Children [
                    :label("Hsl"),
                    (:FeathersColorSwatch SwatchType::Hsl)
                ]
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslHue
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.hue = change.value;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslSaturation
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.saturation = change.value;
                })
            ),
            (
                :FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslLightness
                }
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.lightness = change.value;
                })
            )
        ]
    }
}

fn demo_column_2() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            (
                :pane Children [
                    :pane_header Children [
                        :FeathersToolButton {
                            @variant: ButtonVariant::Primary,
                        } Children [
                            (Text("\u{0398}") ThemedText)
                        ],
                        :pane_header_divider,
                        :FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } Children [
                            (Text("\u{00BC}") ThemedText)
                        ],
                        :FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } Children [
                            (Text("\u{00BD}") ThemedText)
                        ],
                        :FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } Children [
                            (Text("\u{00BE}") ThemedText)
                        ],
                        :pane_header_divider,
                        :FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } Children [
                            :icon(icons::CHEVRON_DOWN)
                        ],
                        :flex_spacer,
                        :FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } Children [
                            :icon(icons::X)
                        ],
                    ],
                    (
                        :pane_body Children [
                            :label_dim("A standard editor pane"),
                            :subpane Children [
                                :subpane_header Children [
                                    (Text("Left") ThemedText),
                                    (Text("Center") ThemedText),
                                    (Text("Right") ThemedText)
                                ],
                                :subpane_body Children [
                                    :label_dim("A standard sub-pane"),
                                    :group
                                    Children [
                                        :group_header Children [
                                            (Text("Group") ThemedText),
                                        ],
                                        :group_body
                                        Children [
                                            :label("A standard group"),
                                            :label_small("Scalar property"),
                                            (
                                                :FeathersNumberInput
                                                DemoScalarField
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    if value_change.is_final {
                                                        states.scalar_prop = value_change.value;
                                                    }
                                                })
                                            ),
                                            :label_small("Scalar property (copy)"),
                                            (
                                                :FeathersNumberInput
                                                DemoScalarField
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    if value_change.is_final {
                                                        states.scalar_prop = value_change.value;
                                                    }
                                                })
                                            ),
                                            :label_small("Vec3 property"),
                                            Node {
                                                display: Display::Flex,
                                                flex_direction: FlexDirection::Row,
                                                column_gap: px(6),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::SpaceBetween,
                                            }
                                            Children [
                                                (
                                                    :FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_X_AXIS,
                                                        @label_text: "X",
                                                    }
                                                    DemoVec3Field::X
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    BorderColor::all(palette::X_AXIS)
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.x = value_change.value;
                                                        }
                                                    })
                                                ),
                                                (
                                                    :FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Y_AXIS,
                                                        @label_text: "Y",
                                                    }
                                                    DemoVec3Field::Y
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.y = value_change.value;
                                                        }
                                                    })
                                                ),
                                                (
                                                    :FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Z_AXIS,
                                                        @label_text: "Z",
                                                    }
                                                    DemoVec3Field::Z
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.z = value_change.value;
                                                        }
                                                    })
                                                ),
                                            ],
                                        ],
                                    ]
                                ],
                            ]
                        ]
                    ),
                ]
            ),
        ]
    }
}

fn update_colors(
    states: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    mut swatches: Query<(&mut ColorSwatchValue, &SwatchType), With<FeathersColorSwatch>>,
    mut color_planes: Query<&mut ColorPlaneValue, With<FeathersColorPlane>>,
    q_text_input: Single<(Entity, &mut EditableText), With<HexColorInput>>,
    q_scalar_input: Query<Entity, With<DemoScalarField>>,
    q_vec3_input: Query<(Entity, &DemoVec3Field)>,
    mut commands: Commands,
    focus: Res<InputFocus>,
) {
    if states.is_changed() {
        for (slider_ent, slider, mut base) in sliders.iter_mut() {
            match slider.channel {
                ColorChannel::Red => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.red));
                }
                ColorChannel::Green => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.green));
                }
                ColorChannel::Blue => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.blue));
                }
                ColorChannel::HslHue => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.hue));
                }
                ColorChannel::HslSaturation => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.saturation));
                }
                ColorChannel::HslLightness => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.lightness));
                }
                ColorChannel::Alpha => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.alpha));
                }
            }
        }

        for (mut swatch_value, swatch_type) in swatches.iter_mut() {
            swatch_value.0 = match swatch_type {
                SwatchType::Rgb => states.rgb_color.into(),
                SwatchType::Hsl => states.hsl_color.into(),
            };
        }

        for mut plane_value in color_planes.iter_mut() {
            plane_value.0.x = states.rgb_color.red;
            plane_value.0.y = states.rgb_color.blue;
            plane_value.0.z = states.rgb_color.green;
        }

        // Only update the hex input field when it's not focused, otherwise it interferes
        // with typing.
        let (input_ent, mut editable_text) = q_text_input.into_inner();
        if Some(input_ent) != focus.get() {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(states.rgb_color.to_hex().into()));
        }

        for scalar_input_ent in q_scalar_input.iter() {
            commands.trigger(UpdateNumberInput {
                entity: scalar_input_ent,
                value: NumberInputValue::F32(states.scalar_prop),
            });
        }

        for (vec3_input_ent, axis) in q_vec3_input.iter() {
            let new_value = match axis {
                DemoVec3Field::X => states.vec3_prop.x,
                DemoVec3Field::Y => states.vec3_prop.y,
                DemoVec3Field::Z => states.vec3_prop.z,
            };

            commands.trigger(UpdateNumberInput {
                entity: vec3_input_ent,
                value: NumberInputValue::F32(new_value),
            });
        }
    }
}

fn handle_hex_color_change(
    _change: On<TextEditChange>,
    q_text_input: Single<&EditableText, With<HexColorInput>>,
    mut colors: ResMut<DemoWidgetStates>,
) {
    let editable_text = *q_text_input;
    if let Ok(color) = Srgba::hex(editable_text.value().to_string())
        && color != colors.rgb_color
    {
        colors.rgb_color = color;
    }
}
