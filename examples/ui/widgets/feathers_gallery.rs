//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    feathers::{
        constants::{fonts, icons},
        containers::{
            flex_spacer, group, group_body, group_header, pane, pane_body, pane_header,
            pane_header_divider, subpane, subpane_body, subpane_header,
        },
        controls::{
            button, checkbox, color_plane, color_slider, color_swatch, disclosure_toggle, menu,
            menu_button, menu_divider, menu_item, menu_popup, radio, slider, text_input,
            text_input_container, toggle_switch, tool_button, ButtonProps, ButtonVariant,
            CheckboxProps, ColorChannel, ColorPlane, ColorPlaneValue, ColorSlider,
            ColorSliderProps, ColorSwatch, ColorSwatchValue, MenuButtonProps, MenuItemProps,
            RadioProps, SliderBaseColor, SliderProps, TextInputProps,
        },
        cursor::{EntityCursor, OverrideCursor},
        dark_theme::create_dark_theme,
        display::{icon, label, label_dim},
        font_styles::InheritableFont,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::{tab_navigation::TabGroup, AutoFocus, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled},
    ui_widgets::{
        checkbox_self_update, slider_self_update, Activate, ActivateOnPress, RadioButton,
        RadioGroup, SliderPrecision, SliderStep, SliderValue, ValueChange,
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
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Normal") ThemedText),
                            )),
                            ..default()
                        })
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Normal button clicked!");
                        })
                        AutoFocus
                    ),
                    (
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Disabled") ThemedText),
                            )),
                            ..default()
                        })
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
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Primary") ThemedText),
                            )),
                            variant: ButtonVariant::Primary,
                            ..default()
                        })
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Disabled button clicked!");
                        })
                    ),
                    (
                        :menu
                        Children [
                            (
                                :menu_button(MenuButtonProps {
                                    caption: Box::new(bsn_list!(
                                        (Text("Menu") ThemedText),
                                    )),
                                    ..default()
                                })
                                Node {
                                    flex_grow: 1.0,
                                }
                            ),
                            (
                                :menu_popup
                                Children [
                                    (
                                        menu_item(MenuItemProps {
                                            caption: Box::new(bsn_list!(
                                                (Text("MenuItem 1") ThemedText)))
                                        })
                                        on(|_: On<Activate>| {
                                            info!("Menu item 1 clicked!");
                                        })
                                    ),
                                    (
                                        menu_item(MenuItemProps {
                                            caption: Box::new(bsn_list!(
                                                (Text("MenuItem 2") ThemedText)))
                                        })
                                        on(|_: On<Activate>| {
                                            info!("Menu item 2 clicked!");
                                        })
                                    ),
                                    :menu_divider,
                                    (
                                        menu_item(MenuItemProps {
                                            caption: Box::new(bsn_list!(
                                                (Text("MenuItem 3") ThemedText)))
                                        })
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
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Left") ThemedText),
                            )),
                            corners: RoundedCorners::Left,
                            ..default()
                        })
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Left button clicked!");
                        })
                    ),
                    (
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Center") ThemedText),
                            )),
                            corners: RoundedCorners::None,
                            ..default()
                        })
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Center button clicked!");
                        })
                    ),
                    (
                        button(ButtonProps {
                            caption: Box::new(bsn_list!(
                                (Text("Right") ThemedText),
                            )),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::Right,
                        })
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
                checkbox(CheckboxProps {
                    caption: Box::new(bsn_list!(
                        (Text("Checkbox") ThemedText),
                    )),
                })
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
                checkbox(CheckboxProps {
                    caption: Box::new(bsn_list!(
                        (Text("Fast Click Checkbox") ThemedText),
                    )),
                })
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
                checkbox(CheckboxProps {
                    caption: Box::new(bsn_list!(
                        (Text("Disabled") ThemedText),
                    )),
                })
                InteractionDisabled
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                checkbox(CheckboxProps {
                    caption: Box::new(bsn_list!(
                        (Text("Checked+Disabled") ThemedText),
                    )),
                })
                InteractionDisabled
                Checked
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
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
                    (radio(RadioProps {
                        caption: Box::new(bsn_list!(
                            (Text("One") ThemedText),
                        )),
                    }) Checked),
                    (radio(RadioProps {
                        caption: Box::new(bsn_list!(
                            (Text("Two") ThemedText),
                        )),
                    })),
                    (radio(RadioProps {
                        caption: Box::new(bsn_list!(
                            (Text("Fast Click") ThemedText),
                        )),
                    }) ActivateOnPress),
                    (radio(RadioProps {
                        caption: Box::new(bsn_list!(
                            (Text("Disabled") ThemedText),
                        )),
                    }) InteractionDisabled),
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
                    (toggle_switch() ActivateOnPress on(checkbox_self_update)),
                    (toggle_switch() InteractionDisabled on(checkbox_self_update)),
                    (toggle_switch() InteractionDisabled Checked on(checkbox_self_update)),
                    (disclosure_toggle() on(checkbox_self_update)),
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
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(4.0),
                }
                Children [
                    :label("Srgba"),
                    // Spacer
                    :flex_spacer,
                    // Text input
                    (
                        :text_input_container
                        Node {
                            flex_grow: 0.
                            padding: { px(4.).left().with_right(px(0.)) },
                        }
                        Children [
                            (
                                text_input(TextInputProps {
                                    visible_width: Some(10.),
                                    max_characters: Some(9),
                                })
                                InheritableFont {
                                    font: fonts::MONO
                                }
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
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                }
                Children [
                    :label("Hsl"),
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
    }
}

fn demo_column_2() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(8.0),
            width: Val::Percent(30.),
            min_width: Val::Px(200.),
        }
        Children [
            (
                :pane Children [
                    :pane_header Children [
                        :tool_button(ButtonProps {
                            variant: ButtonVariant::Primary,
                            ..default()
                        }) Children [
                            (Text("\u{0398}") ThemedText)
                        ],
                        :pane_header_divider,
                        :tool_button(ButtonProps{
                            variant: ButtonVariant::Plain,
                            ..default()
                        }) Children [
                            (Text("\u{00BC}") ThemedText)
                        ],
                        :tool_button(ButtonProps{
                            variant: ButtonVariant::Plain,
                            ..default()
                        }) Children [
                            (Text("\u{00BD}") ThemedText)
                        ],
                        :tool_button(ButtonProps{
                            variant: ButtonVariant::Plain,
                            ..default()
                        }) Children [
                            (Text("\u{00BE}") ThemedText)
                        ],
                        :pane_header_divider,
                        :tool_button(ButtonProps{
                            variant: ButtonVariant::Plain,
                            ..default()
                        }) Children [
                            :icon(icons::CHEVRON_DOWN)
                        ],
                        :flex_spacer,
                        :tool_button(ButtonProps{
                            variant: ButtonVariant::Plain,
                            ..default()
                        }) Children [
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
                                    :group Children [
                                        :group_header Children [
                                            (Text("Group") ThemedText),
                                        ],
                                        :group_body Children [
                                            :label_dim("A standard group"),
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
        if Some(input_ent) != focus.get() {
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
    if let Ok(color) = Srgba::hex(editable_text.value().to_string())
        && color != colors.rgb_color
    {
        colors.rgb_color = color;
    }
}
