//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    feathers::{
        controls::{
            button, checkbox, color_plane, color_slider, color_swatch, radio, slider, text_input,
            text_input_container, toggle_switch, ButtonProps, ButtonVariant, ColorChannel,
            ColorPlane, ColorPlaneValue, ColorSlider, ColorSliderProps, ColorSwatch,
            ColorSwatchValue, SliderBaseColor, SliderProps, TextInputProps,
        },
        cursor::{EntityCursor, OverrideCursor},
        dark_theme::create_dark_theme,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::{tab_navigation::TabGroup, AutoFocus, InputFocus},
    prelude::*,
    scene::prelude::Scene,
    text::{EditableText, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled},
    ui_widgets::{
        checkbox_self_update, slider_self_update, Activate, RadioButton, RadioGroup,
        SliderPrecision, SliderStep, SliderValue, ValueChange,
    },
    window::SystemCursorIcon,
};

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
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

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(DemoWidgetStates {
            rgb_color: palettes::tailwind::EMERALD_800.with_alpha(0.7),
            hsl_color: palettes::tailwind::AMBER_800.into(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update_colors)
        .run();
}

fn setup(world: &mut World) -> Result {
    world.spawn_scene_list(bsn_list![Camera2d, demo_root()])?;
    Ok(())
}

fn demo_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(px(8)),
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
                            button(ButtonProps::default())
                            on(|_activate: On<Activate>| {
                                info!("Normal button clicked!");
                            })
                            AutoFocus
                            Children [ (Text::new("Normal") ThemedText) ]
                        ),
                        (
                            button(ButtonProps::default())
                            InteractionDisabled
                            DemoDisabledButton
                            on(|_activate: On<Activate>| {
                                info!("Disabled button clicked!");
                            })
                            Children [ (Text::new("Disabled") ThemedText) ]
                        ),
                        (
                            button(ButtonProps {
                                variant: ButtonVariant::Primary,
                                ..default()
                            })
                            on(|_activate: On<Activate>| {
                                info!("Disabled button clicked!");
                            })
                            Children [ (Text::new("Primary") ThemedText) ]
                        ),
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
                            button(ButtonProps {
                                corners: RoundedCorners::Left,
                                ..default()
                            })
                            on(|_activate: On<Activate>| {
                                info!("Left button clicked!");
                            })
                            Children [ (Text::new("Left") ThemedText) ]
                        ),
                        (
                            button(ButtonProps {
                                corners: RoundedCorners::None,
                                ..default()
                            })
                            on(|_activate: On<Activate>| {
                                info!("Center button clicked!");
                            })
                            Children [ (Text::new("Center") ThemedText) ]
                        ),
                        (
                            button(ButtonProps {
                                variant: ButtonVariant::Primary,
                                corners: RoundedCorners::Right,
                            })
                            on(|_activate: On<Activate>| {
                                info!("Right button clicked!");
                            })
                            Children [ (Text::new("Right") ThemedText) ]
                        ),
                    ]
                ),
                (
                    button(ButtonProps::default())
                    on(|_activate: On<Activate>, mut ovr: ResMut<OverrideCursor>| {
                        ovr.0 = if ovr.0.is_some() {
                            None
                        } else {
                            Some(EntityCursor::System(SystemCursorIcon::Wait))
                        };
                        info!("Override cursor button clicked!");
                    })
                    Children [ (Text::new("Toggle override") ThemedText) ]
                ),
                (
                    checkbox()
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
                    Children [ (Text::new("Checkbox") ThemedText) ]
                ),
                (
                    checkbox()
                    InteractionDisabled
                    on(|_change: On<ValueChange<bool>>| {
                        warn!("Disabled checkbox clicked!");
                    })
                    Children [ (Text::new("Disabled") ThemedText) ]
                ),
                (
                    checkbox()
                    InteractionDisabled
                    Checked
                    on(|_change: On<ValueChange<bool>>| {
                        warn!("Disabled checkbox clicked!");
                    })
                    Children [ (Text::new("Disabled+Checked") ThemedText) ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    RadioGroup
                    on(
                        |value_change: On<ValueChange<Entity>>,
                         q_radio: Query<Entity, With<RadioButton>>,
                         mut commands: Commands| {
                            for radio in q_radio.iter() {
                                if radio == value_change.value {
                                    commands.entity(radio).insert(Checked);
                                } else {
                                    commands.entity(radio).remove::<Checked>();
                                }
                            }
                        }
                    )
                    Children [
                        (radio() Checked Children [ (Text::new("One") ThemedText) ]),
                        (radio() Children [ (Text::new("Two") ThemedText) ]),
                        (radio() Children [ (Text::new("Three") ThemedText) ]),
                        (radio() InteractionDisabled Children [ (Text::new("Disabled") ThemedText) ]),
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
                        (toggle_switch() on(checkbox_self_update)),
                        (toggle_switch() InteractionDisabled on(checkbox_self_update)),
                        (toggle_switch() InteractionDisabled Checked on(checkbox_self_update)),
                    ]
                ),
                (
                    slider(SliderProps {
                        max: 100.0,
                        value: 20.0,
                        ..default()
                    })
                    SliderStep(10.)
                    SliderPrecision(2)
                    on(slider_self_update)
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: px(4.0),
                    }
                    Children [
                        Text("Srgba"),
                        // Spacer
                        Node {
                            flex_grow: 1.0,
                        },
                        // Text input
                        (
                            :text_input_container
                            Node {
                                flex_grow: 1.0,
                            }
                            Children [
                                (
                                    text_input(TextInputProps {
                                        max_characters: Some(9),
                                    })
                                    HexColorInput
                                    on(handle_hex_color_change)
                                )
                            ]
                        )
                        (color_swatch() SwatchType::Rgb),
                    ]
                ),
                (
                    color_plane(ColorPlane::RedBlue)
                    on(|change: On<ValueChange<Vec2>>, mut color: ResMut<DemoWidgetStates>| {
                        color.rgb_color.red = change.value.x;
                        color.rgb_color.blue = change.value.y;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::Red
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.rgb_color.red = change.value;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::Green
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.rgb_color.green = change.value;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::Blue
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.rgb_color.blue = change.value;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::Alpha
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.rgb_color.alpha = change.value;
                    })
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                    }
                    Children [
                        Text("Hsl"),
                        (color_swatch() SwatchType::Hsl)
                    ]
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::HslHue
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.hsl_color.hue = change.value;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::HslSaturation
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.hsl_color.saturation = change.value;
                    })
                ),
                (
                    color_slider(ColorSliderProps {
                        value: 0.5,
                        channel: ColorChannel::HslLightness
                    })
                    on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                        color.hsl_color.lightness = change.value;
                    })
                )
            ]
        )]
    }
}

fn update_colors(
    colors: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    mut swatches: Query<(&mut ColorSwatchValue, &SwatchType), With<ColorSwatch>>,
    mut color_planes: Query<&mut ColorPlaneValue, With<ColorPlane>>,
    q_text_input: Single<(Entity, &mut EditableText), With<HexColorInput>>,
    mut commands: Commands,
    focus: Res<InputFocus>,
) {
    if colors.is_changed() {
        for (slider_ent, slider, mut base) in sliders.iter_mut() {
            match slider.channel {
                ColorChannel::Red => {
                    base.0 = colors.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.rgb_color.red));
                }
                ColorChannel::Green => {
                    base.0 = colors.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.rgb_color.green));
                }
                ColorChannel::Blue => {
                    base.0 = colors.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.rgb_color.blue));
                }
                ColorChannel::HslHue => {
                    base.0 = colors.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.hsl_color.hue));
                }
                ColorChannel::HslSaturation => {
                    base.0 = colors.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.hsl_color.saturation));
                }
                ColorChannel::HslLightness => {
                    base.0 = colors.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.hsl_color.lightness));
                }
                ColorChannel::Alpha => {
                    base.0 = colors.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(colors.rgb_color.alpha));
                }
            }
        }

        for (mut swatch_value, swatch_type) in swatches.iter_mut() {
            swatch_value.0 = match swatch_type {
                SwatchType::Rgb => colors.rgb_color.into(),
                SwatchType::Hsl => colors.hsl_color.into(),
            };
        }

        for mut plane_value in color_planes.iter_mut() {
            plane_value.0.x = colors.rgb_color.red;
            plane_value.0.y = colors.rgb_color.blue;
            plane_value.0.z = colors.rgb_color.green;
        }

        // Only update the hex input field when it's not focused, otherwise it interferes
        // with typing.
        let (input_ent, mut editable_text) = q_text_input.into_inner();
        if Some(input_ent) != focus.0 {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(colors.rgb_color.to_hex().into()));
        }
    }
}

fn handle_hex_color_change(
    _change: On<TextEditChange>,
    q_text_input: Single<&EditableText, With<HexColorInput>>,
    mut colors: ResMut<DemoWidgetStates>,
) {
    let editable_text = *q_text_input;
    if let Ok(color) = Srgba::hex(editable_text.value().to_string()) {
        colors.rgb_color = color;
    }
}
