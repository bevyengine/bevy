//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    core_widgets::{
        callback, Activate, CoreRadio, CoreRadioGroup, CoreWidgetsPlugins, SliderPrecision,
        SliderStep, SliderValue, ValueChange,
    },
    feathers::{
        containers::{
            flex_spacer, pane, pane_body, pane_header, pane_header_divider, subpane, subpane_body,
            subpane_header,
        },
        controls::{
            button, checkbox, color_slider, color_swatch, radio, slider, toggle_switch,
            tool_button, ButtonProps, ButtonVariant, CheckboxProps, ColorChannel, ColorSlider,
            ColorSliderProps, ColorSwatch, SliderBaseColor, SliderProps, ToggleSwitchProps,
        },
        dark_theme::create_dark_theme,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugin,
    },
    input_focus::{
        tab_navigation::{TabGroup, TabNavigationPlugin},
        InputDispatchPlugin,
    },
    prelude::*,
    scene2::prelude::{Scene, *},
    ui::{Checked, InteractionDisabled},
    winit::WinitSettings,
};
use bevy_ecs::VariantDefaults;

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
}

#[derive(Component, Clone, Copy, PartialEq, Default, VariantDefaults)]
enum SwatchType {
    #[default]
    Rgb,
    Hsl,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            FeathersPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(DemoWidgetStates {
            rgb_color: palettes::tailwind::EMERALD_800.with_alpha(0.7),
            hsl_color: palettes::tailwind::AMBER_800.into(),
        })
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, update_colors)
        .run();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn_scene(demo_root());
}

fn demo_root() -> impl Scene {
    bsn! {
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(8.0),
                width: Val::Percent(30.),
                min_width: Val::Px(200.),
            } [
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: Val::Px(8.0),
                } [
                    (
                        :button(ButtonProps {
                            on_click: callback(|_: In<Activate>| {
                                info!("Normal button clicked!");
                            }),
                            ..default()
                        }) [(Text("Normal") ThemedText)]
                    ),
                    (
                        :button(
                            ButtonProps {
                                on_click: callback(|_: In<Activate>| {
                                    info!("Disabled button clicked!");
                                }),
                                ..default()
                            },
                        )
                        InteractionDisabled::default()
                        [(Text("Disabled") ThemedText)]
                    ),
                    (
                        :button(
                            ButtonProps {
                                on_click: callback(|_: In<Activate>| {
                                    info!("Primary button clicked!");
                                }),
                                variant: ButtonVariant::Primary,
                                ..default()
                            },
                        ) [(Text("Primary") ThemedText)]
                    ),
                ],
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: Val::Px(1.0),
                } [
                    (
                        :button(ButtonProps {
                            on_click: callback(|_: In<Activate>| {
                                info!("Left button clicked!");
                            }),
                            corners: RoundedCorners::Left,
                            ..default()
                        }) [(Text("Left") ThemedText)]
                    ),
                    (
                        :button(ButtonProps {
                            on_click: callback(|_: In<Activate>| {
                                info!("Center button clicked!");
                            }),
                            corners: RoundedCorners::None,
                            ..default()
                        }) [(Text("Center") ThemedText)]
                    ),
                    (
                        :button(ButtonProps {
                            on_click: callback(|_: In<Activate>| {
                                info!("Right button clicked!");
                            }),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::Right,
                        }) [(Text("Right") ThemedText)]
                    ),
                ],
                :button(
                    ButtonProps {
                        on_click: callback(|_: In<Activate>| {
                            info!("Wide button clicked!");
                        }),
                        ..default()
                    }
                ) [(Text("Button") ThemedText)],
                (
                    :checkbox(CheckboxProps::default())
                    Checked::default()
                    [(Text("Checkbox") ThemedText)]
                ),
                (
                    :checkbox(CheckboxProps::default())
                    InteractionDisabled::default()
                    [(Text("Disabled") ThemedText)]
                ),
                (
                    :checkbox(CheckboxProps::default())
                    InteractionDisabled
                    Checked::default()
                    [(Text("Disabled+Checked") ThemedText)]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                    }
                    CoreRadioGroup {
                        // Update radio button states based on notification from radio group.
                        on_change: callback(
                            |ent: In<Activate>, q_radio: Query<Entity, With<CoreRadio>>, mut commands: Commands| {
                                for radio in q_radio.iter() {
                                    if radio == ent.0.0 {
                                        commands.entity(radio).insert(Checked);
                                    } else {
                                        commands.entity(radio).remove::<Checked>();
                                    }
                                }
                            },
                        ),
                    }
                    [
                        :radio Checked::default() [(Text("One") ThemedText)],
                        :radio [(Text("Two") ThemedText)],
                        :radio [(Text("Three") ThemedText)],
                        :radio InteractionDisabled::default() [(Text("Disabled") ThemedText)],
                    ]
                ),
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: Val::Px(8.0),
                } [
                    :toggle_switch(ToggleSwitchProps::default()),
                    :toggle_switch(ToggleSwitchProps::default()) InteractionDisabled,
                    :toggle_switch(ToggleSwitchProps::default()) InteractionDisabled Checked,
                ],
                Node {
                    flex_direction: FlexDirection::Column,
                } [
                    (
                        :slider(SliderProps {
                            max: 100.0,
                            value: 20.0,
                            ..default()
                        })
                        SliderStep(10.)
                        SliderPrecision(2)
                    ),
                    (
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                        }
                        [Text("Srgba"), (color_swatch() SwatchType::Rgb)]
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.rgb_color.red = change.value;
                                },
                            ),
                            channel: ColorChannel::Red
                        },
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.rgb_color.green = change.value;
                                },
                            ),
                            channel: ColorChannel::Green
                        },
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.rgb_color.blue = change.value;
                                },
                            ),
                            channel: ColorChannel::Blue
                        },
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.rgb_color.alpha = change.value;
                                },
                            ),
                            channel: ColorChannel::Alpha
                        },
                    ),
                    (
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                        }
                        [Text("Hsl"), (color_swatch() SwatchType::Hsl)]
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.hsl_color.hue = change.value;
                                },
                            ),
                            channel: ColorChannel::HslHue
                        },
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.hsl_color.saturation = change.value;
                                },
                            ),
                            channel: ColorChannel::HslSaturation
                        },
                    ),
                    :color_slider(
                        ColorSliderProps {
                            value: 0.5,
                            on_change: callback(
                                |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                                    color.hsl_color.lightness = change.value;
                                },
                            ),
                            channel: ColorChannel::HslLightness
                        },
                    ),
                    color_swatch(),
                ]
            ],
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(8.0),
                width: Val::Percent(30.),
                min_width: Val::Px(200.),
            } [
                (
                    :subpane [
                        :subpane_header [
                            (Text("Left") ThemedText),
                            (Text("Center") ThemedText),
                            (Text("Right") ThemedText)
                        ],
                        :subpane_body [
                            (Text("Body") ThemedText),
                        ],
                    ]
                ),
                (
                    :pane [
                        :pane_header [
                            :tool_button(ButtonProps {
                                variant: ButtonVariant::Selected,
                                ..default()
                            }) [
                                (Text("\u{0398}") ThemedText)
                            ],
                            :pane_header_divider,
                            :tool_button(ButtonProps{
                                variant: ButtonVariant::Plain,
                                ..default()
                            }) [
                                (Text("\u{00BC}") ThemedText)
                            ],
                            :tool_button(ButtonProps{
                                variant: ButtonVariant::Plain,
                                ..default()
                            }) [
                                (Text("\u{00BD}") ThemedText)
                            ],
                            :tool_button(ButtonProps{
                                variant: ButtonVariant::Plain,
                                ..default()
                            }) [
                                (Text("\u{00BE}") ThemedText)
                            ],
                            :pane_header_divider,
                            :tool_button(ButtonProps{
                                variant: ButtonVariant::Plain,
                                ..default()
                            }) [
                                (Text("\u{20AC}") ThemedText)
                            ],
                            :flex_spacer,
                            :tool_button(ButtonProps{
                                variant: ButtonVariant::Plain,
                                ..default()
                            }) [
                                (Text("\u{00D7}") ThemedText)
                            ],
                        ],
                        (
                            :pane_body [
                                (Text("Some") ThemedText),
                                (Text("Content") ThemedText),
                                (Text("Here") ThemedText),
                            ]
                            BackgroundColor(palettes::tailwind::EMERALD_800)
                        ),
                    ]
                )
            ]
        ]
    }
}

fn update_colors(
    colors: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    new_sliders: Query<(), Added<ColorSlider>>,
    swatches: Query<(&SwatchType, &Children), With<ColorSwatch>>,
    mut commands: Commands,
) {
    if colors.is_changed() || !new_sliders.is_empty() {
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

        for (swatch_type, children) in swatches.iter() {
            commands
                .entity(children[0])
                .insert(BackgroundColor(match swatch_type {
                    SwatchType::Rgb => colors.rgb_color.into(),
                    SwatchType::Hsl => colors.hsl_color.into(),
                }));
        }
    }
}
