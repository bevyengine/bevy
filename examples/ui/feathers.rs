//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    core_widgets::CoreWidgetsPlugin,
    ecs::system::SystemId,
    feathers::{
        controls::{button, slider, ButtonProps, ButtonVariant, SliderProps},
        dark::create_dark_theme,
        theme::{self, corners::RoundedCorners, ThemeBackgroundColor, UiTheme, UseTheme},
        FeathersPlugin,
    },
    input_focus::{tab_navigation::TabGroup, InputDispatchPlugin},
    prelude::*,
    ui::InteractionDisabled,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugin,
            InputDispatchPlugin,
            FeathersPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // System to print a value when the button is clicked.
    let on_click = commands.register_system(|| {
        info!("Button clicked!");
    });

    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(on_click));
}

fn demo_root(on_click: SystemId) -> impl Bundle {
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
        ThemeBackgroundColor(theme::tokens::WINDOW_BG),
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
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Normal"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::All,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Disabled"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::All,
                            overrides: (InteractionDisabled),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Primary"), UseTheme)),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::All,
                            overrides: (),
                            // ..Default::default()
                        }),
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
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Left"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::Left,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Center"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::None,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Right"), UseTheme)),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::Right,
                            overrides: (),
                            // ..Default::default()
                        }),
                    ]
                ),
                button(ButtonProps {
                    on_click: Some(on_click),
                    children: Spawn((Text::new("Button"), UseTheme)),
                    variant: ButtonVariant::Normal,
                    corners: RoundedCorners::All,
                    overrides: (),
                    // ..default()
                }),
                slider(SliderProps {
                    max: 100.0,
                    value: 20.0,
                    precision: 1,
                    ..default()
                }),
            ]
        ),],
    )
}
