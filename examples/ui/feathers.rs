//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    core_widgets::{CoreWidgetsPlugin, SliderStep},
    feathers::{
        controls::{button, slider, ButtonProps, ButtonVariant, SliderProps},
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
    ui::InteractionDisabled,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugin,
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
    let root = demo_root(&mut commands);
    commands.spawn(root);
}

fn demo_root(commands: &mut Commands) -> impl Bundle {
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
                                on_click: Some(commands.register_system(|| {
                                    info!("Normal button clicked!");
                                })),
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Normal"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Some(commands.register_system(|| {
                                    info!("Disabled button clicked!");
                                })),
                                ..default()
                            },
                            InteractionDisabled,
                            Spawn((Text::new("Disabled"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Some(commands.register_system(|| {
                                    info!("Primary button clicked!");
                                })),
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
                                on_click: Some(commands.register_system(|| {
                                    info!("Left button clicked!");
                                })),
                                corners: RoundedCorners::Left,
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Left"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Some(commands.register_system(|| {
                                    info!("Center button clicked!");
                                })),
                                corners: RoundedCorners::None,
                                ..default()
                            },
                            (),
                            Spawn((Text::new("Center"), ThemedText))
                        ),
                        button(
                            ButtonProps {
                                on_click: Some(commands.register_system(|| {
                                    info!("Right button clicked!");
                                })),
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
                        on_click: Some(commands.register_system(|| {
                            info!("Wide button clicked!");
                        })),
                        ..default()
                    },
                    (),
                    Spawn((Text::new("Button"), ThemedText))
                ),
                slider(
                    SliderProps {
                        max: 100.0,
                        value: 20.0,
                        ..default()
                    },
                    SliderStep(10.)
                ),
            ]
        ),],
    )
}
