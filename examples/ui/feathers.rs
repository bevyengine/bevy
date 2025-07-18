//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    core_widgets::{
        callback, Activate, CoreRadio, CoreRadioGroup, CoreWidgetsPlugins, SliderPrecision,
        SliderStep,
    },
    feathers::{
        containers::{
            flex_spacer, pane, pane_body, pane_header, pane_header_divider, subpane, subpane_body,
            subpane_header,
        },
        controls::{
            button, checkbox, radio, slider, toggle_switch, tool_button, ButtonProps,
            ButtonVariant, CheckboxProps, SliderProps, ToggleSwitchProps,
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
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
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
                (
                    :slider(SliderProps {
                        max: 100.0,
                        value: 20.0,
                        ..default()
                    })
                    SliderStep(10.)
                    SliderPrecision(2)
                ),
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
