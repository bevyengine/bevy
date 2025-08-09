//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    core_widgets::{
        Activate, Callback, CoreRadio, CoreRadioGroup, CoreWidgetsPlugins, SliderPrecision,
        SliderStep, SliderValue, ValueChange,
    },
    feathers::{
        controls::{
            button, checkbox, color_slider, color_swatch, radio, slider, toggle_switch,
            ButtonProps, ButtonVariant, CheckboxProps, ColorChannel, ColorSlider, ColorSliderProps,
            ColorSwatch, SliderBaseColor, SliderProps, ToggleSwitchProps,
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
    ui::{Checked, InteractionDisabled},
    winit::WinitSettings,
};

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum SwatchType {
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
    let root = demo_root(&mut commands);
    commands.spawn(root);
}

fn demo_root(commands: &mut Commands) -> impl Bundle {
    // Update radio button states based on notification from radio group.
    let radio_exclusion = commands.register_system(
        |ent: In<Activate>, q_radio: Query<Entity, With<CoreRadio>>, mut commands: Commands| {
            for radio in q_radio.iter() {
                if radio == ent.0 .0 {
                    commands.entity(radio).insert(Checked);
                } else {
                    commands.entity(radio).remove::<Checked>();
                }
            }
        },
    );

    let change_red = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.rgb_color.red = change.value;
        },
    );

    let change_green = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.rgb_color.green = change.value;
        },
    );

    let change_blue = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.rgb_color.blue = change.value;
        },
    );

    let change_alpha = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.rgb_color.alpha = change.value;
        },
    );

    let change_hue = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.hsl_color.hue = change.value;
        },
    );

    let change_saturation = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.hsl_color.saturation = change.value;
        },
    );

    let change_lightness = commands.register_system(
        |change: In<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
            color.hsl_color.lightness = change.value;
        },
    );

    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        TabGroup::default(),
        ThemeBackgroundColor(tokens::WINDOW_BG),
        children![(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(8.0),
                width: Val::Percent(30.),
                min_width: Val::Px(200.),
                ..default()
            },
            children![
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    children![
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Normal button clicked!");
                                    }
                                )),
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Normal"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Disabled button clicked!");
                                    }
                                )),
                                ..default()
                            },
                            InteractionDisabled,
                            Spawn((Text::new("Disabled"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Primary button clicked!");
                                    }
                                )),
                                variant: ButtonVariant::Primary,
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Primary"), ThemedText))
                        ),
                    ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        column_gap: Val::Px(1.0),
                        ..default()
                    },
                    children![
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Left button clicked!");
                                    }
                                )),
                                corners: RoundedCorners::Left,
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Left"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Center button clicked!");
                                    }
                                )),
                                corners: RoundedCorners::None,
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Center"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Callback::System(commands.register_system(
                                    |_: In<Activate>| {
                                        info!("Right button clicked!");
                                    }
                                )),
                                variant: ButtonVariant::Primary,
                                corners: RoundedCorners::Right,
                            },
                            (),
                            Spawn((Text::new("Right"), ThemedText))
                        ),
                    ]
                ),
                button(
                    ButtonProps {
                        on_click: Callback::System(commands.register_system(|_: In<Activate>| {
                            info!("Wide button clicked!");
                        })),
                        ..default()
                    },
                    (),
                    Spawn((Text::new("Button"), ThemedText))
                ),
                checkbox(
                    CheckboxProps {
                        on_change: Callback::Ignore,
                    },
                    Checked,
                    Spawn((Text::new("Checkbox"), ThemedText))
                ),
                checkbox(
                    CheckboxProps {
                        on_change: Callback::Ignore,
                    },
                    InteractionDisabled,
                    Spawn((Text::new("Disabled"), ThemedText))
                ),
                checkbox(
                    CheckboxProps {
                        on_change: Callback::Ignore,
                    },
                    (InteractionDisabled, Checked),
                    Spawn((Text::new("Disabled+Checked"), ThemedText))
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    CoreRadioGroup {
                        on_change: Callback::System(radio_exclusion),
                    },
                    children![
                        radio(Checked, Spawn((Text::new("One"), ThemedText))),
                        radio((), Spawn((Text::new("Two"), ThemedText))),
                        radio((), Spawn((Text::new("Three"), ThemedText))),
                        radio(
                            InteractionDisabled,
                            Spawn((Text::new("Disabled"), ThemedText))
                        ),
                    ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    children![
                        toggle_switch(
                            ToggleSwitchProps {
                                on_change: Callback::Ignore,
                            },
                            (),
                        ),
                        toggle_switch(
                            ToggleSwitchProps {
                                on_change: Callback::Ignore,
                            },
                            InteractionDisabled,
                        ),
                        toggle_switch(
                            ToggleSwitchProps {
                                on_change: Callback::Ignore,
                            },
                            (InteractionDisabled, Checked),
                        ),
                    ]
                ),
                slider(
                    SliderProps {
                        max: 100.0,
                        value: 20.0,
                        ..default()
                    },
                    (SliderStep(10.), SliderPrecision(2)),
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    children![Text("Srgba".to_owned()), color_swatch(SwatchType::Rgb),]
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_red),
                        channel: ColorChannel::Red
                    },
                    ()
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_green),
                        channel: ColorChannel::Green
                    },
                    ()
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_blue),
                        channel: ColorChannel::Blue
                    },
                    ()
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_alpha),
                        channel: ColorChannel::Alpha
                    },
                    ()
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    children![Text("Hsl".to_owned()), color_swatch(SwatchType::Hsl),]
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_hue),
                        channel: ColorChannel::HslHue
                    },
                    ()
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_saturation),
                        channel: ColorChannel::HslSaturation
                    },
                    ()
                ),
                color_slider(
                    ColorSliderProps {
                        value: 0.5,
                        on_change: Callback::System(change_lightness),
                        channel: ColorChannel::HslLightness
                    },
                    ()
                )
            ]
        ),],
    )
}

fn update_colors(
    colors: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    swatches: Query<(&SwatchType, &Children), With<ColorSwatch>>,
    mut commands: Commands,
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
