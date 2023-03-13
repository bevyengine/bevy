//! Demonstrates how CSS Grid layout can be used to lay items out in a 2D grid
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

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                /// Use the CSS Grid algorithm for laying out this node
                display: Display::Grid,
                /// Make node fill the entirety it's parent (in this case the window)
                size: Size::all(Val::Percent(100.)),
                /// Set a 20px gap/gutter between both rows and columns
                gap: Size::all(Val::Px(20.)),
                /// Set the grid to have 3 columns with sizes [minmax(0, 1fr), minmax(0, 2fr), minmax(0, 1fr)]
                /// This means that the columns with initially have zero size. They will then expand to take up 
                /// the remaining available space in proportion to thier "flex fractions" (fr values).
                ///
                /// The sum of the fr values is 4, so in this case:
                ///   - The 1st column will take 1/4 of the width
                ///   - The 2nd column will take up 2/4 = 1/2 of the width
                ///   - The 3rd column will be 1/4 of the width
                grid_template_columns: vec![GridTrack::flex(1.), GridTrack::flex(2.), GridTrack::flex(1.)],
                /// Set the grid to have 3 rows with sizes [auto, 150px, minmax(0, 1fr)]
                grid_template_rows: vec![GridTrack::auto(), GridTrack::px(150.), GridTrack::flex(1.)],
                ..default()
            },
            background_color: BackgroundColor(Color::WHITE),
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
                    builder.spawn(rect(Color::CYAN));
                    builder.spawn(rect(Color::ORANGE_RED));
                    builder.spawn(rect(Color::DARK_GREEN));
                    builder.spawn(rect(Color::FUCHSIA));
                    builder.spawn(rect(Color::GOLD));

                    builder.spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(10.),
                            left: Val::Px(10.),
                            size: Size::all(Val::Px(40.)),
                            grid_row: GridPlacement::start(1),
                            grid_column: GridPlacement::start(1),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    });
                });

            builder.spawn(rect(Color::GREEN));
        });
}

/// Create a coloured rectangle node. The node has size as it is assumed that it will be
/// spawned as a child of a Grid container with AlignItems::Stretch and JustifyItems::Stretch
/// which will allow it to take it's size from the size of the grid area it occupies.
fn rect(color: Color) -> NodeBundle {
    NodeBundle {
        background_color: BackgroundColor(color),
        ..default()
    }
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
