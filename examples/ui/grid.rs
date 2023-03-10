//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::prelude::*;

const ALIGN_ITEMS_COLOR: Color = Color::rgb(1., 0.066, 0.349);
const MARGIN: Val = Val::Px(5.);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [800., 600.].into(),
                title: "Bevy CSS Grid Layout Example".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(spawn_layout)
        .run();
}

fn rect(color: Color) -> NodeBundle {
    NodeBundle {
        background_color: BackgroundColor(color),
        ..default()
    }
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                size: Size::all(Val::Percent(100.)),
                grid_template_columns: vec![GridTrack::flex(1.), GridTrack::flex(2.), GridTrack::flex(1.)],
                grid_template_rows: vec![GridTrack::auto(), GridTrack::px(150.), GridTrack::flex(1.)],
                gap: Size::all(Val::Px(20.)),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(rect(Color::BLACK));
            builder.spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|builder| {

                    spawn_nested_text_bundle(
                        builder,
                        font.clone(),
                        ALIGN_ITEMS_COLOR,
                        UiRect::right(MARGIN),
                        "hello world",
                    );

                    const REALLY_LONG_PARAGRAPH : &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
                    spawn_nested_text_bundle(
                        builder,
                        font.clone(),
                        ALIGN_ITEMS_COLOR,
                        UiRect::right(MARGIN),
                        REALLY_LONG_PARAGRAPH,
                    );
                });

            builder.spawn(rect(Color::ALICE_BLUE));
            builder.spawn(rect(Color::ANTIQUE_WHITE));
            builder.spawn(rect(Color::AQUAMARINE));
            builder.spawn(rect(Color::AZURE));
            builder.spawn(rect(Color::DARK_GREEN));


            builder.spawn(NodeBundle {
                        style: Style {
                            display: Display::Grid,
                            grid_template_columns: vec![GridTrack::flex(1.), GridTrack::flex(2.), GridTrack::flex(1.)],
                            grid_template_rows: vec![GridTrack::flex(1.), GridTrack::percent(50.), GridTrack::flex(1.)],
                            ..default()
                        },
                        ..default()
                    })
                .with_children(|builder| {
                    builder.spawn(rect(Color::ORANGE));
                    builder.spawn(rect(Color::BISQUE));
                    builder.spawn(rect(Color::BLUE));
                    builder.spawn(rect(Color::CRIMSON));

                    // .with_styled_child(button("Increment").on_press(Message::Increment), |style| {
                    //     style.align_self = Some(AlignSelf::Center);
                    //     style.justify_self = Some(AlignSelf::Center);
                    // })
                    builder.spawn(rect(Color::CYAN));

                    builder.spawn(rect(Color::ORANGE_RED));
                    builder.spawn(rect(Color::DARK_GREEN));
                    builder.spawn(rect(Color::FUCHSIA));
                    builder.spawn(rect(Color::GOLD));

                    builder.spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            grid_row: GridPlacement::start(1),
                            grid_column: GridPlacement::start(1),
                            position: UiRect {
                                left: Val::Px(10.),
                                top: Val::Px(10.),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                });

            builder.spawn(rect(Color::GREEN));
        });
}

fn spawn_nested_text_bundle(
    builder: &mut ChildBuilder,
    font: Handle<Font>,
    background_color: Color,
    margin: UiRect,
    text: &str,
) {
    builder
        .spawn(NodeBundle {
            style: Style {
                margin,
                padding: UiRect {
                    top: Val::Px(1.),
                    left: Val::Px(5.),
                    right: Val::Px(5.),
                    bottom: Val::Px(1.),
                },
                min_size: Size::width(Val::Px(0.)),
                ..default()
            },
            background_color: BackgroundColor(background_color),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font,
                    font_size: 24.0,
                    color: Color::BLACK,
                },
            ));
        });
}
