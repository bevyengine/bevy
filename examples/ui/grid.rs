//! Demonstrates how CSS Grid layout can be used to lay items out in a 2D grid
use bevy::prelude::*;

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

    // Top-level grid (app frame)
    commands
        .spawn(NodeBundle {
            style: Style {
                /// Use the CSS Grid algorithm for laying out this node
                display: Display::Grid,
                /// Make node fill the entirety it's parent (in this case the window)
                size: Size::all(Val::Percent(100.)),
                /// Set a 20px gap/gutter between both rows and columns
                // gap: Size::all(Val::Px(20.)),
                /// Set the grid to have 3 columns with sizes [minmax(0, 1fr), minmax(0, 2fr), minmax(0, 1fr)]
                /// This means that the columns with initially have zero size. They will then expand to take up
                /// the remaining available space in proportion to thier "flex fractions" (fr values).
                ///
                /// The sum of the fr values is 4, so in this case:
                ///   - The 1st column will take 1/4 of the width
                ///   - The 2nd column will take up 2/4 = 1/2 of the width
                ///   - The 3rd column will be 1/4 of the width
                grid_template_columns: vec![GridTrack::min_content(), GridTrack::fr(1.0)],
                /// Set the grid to have 3 rows with sizes [auto, 150px, minmax(0, 1fr)]
                grid_template_rows: vec![
                    GridTrack::auto(),
                    GridTrack::flex(1.0),
                    GridTrack::px(20.),
                ],
                ..default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..default()
        })
        .with_children(|builder| {
            // Header
            builder
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        /// Make this node span two grid column so that it takes up the entire top tow
                        grid_column: GridPlacement::span(2),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|builder| {
                    spawn_nested_text_bundle(builder, font.clone(), "Bevy CSS Grid Layout Example");
                });

            // Main content grid
            builder
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        padding: UiRect::all(Val::Px(24.0)),
                        grid_template_columns: vec![GridTrack::flex::<GridTrack>(1.0).repeat(4)],
                        grid_template_rows: vec![GridTrack::flex::<GridTrack>(1.0).repeat(4)],
                        gap: Size::all(Val::Px(12.0)),
                        aspect_ratio: Some(1.0),
                        size: Size::height(Val::Percent(100.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::DARK_GRAY),
                    ..default()
                })
                .with_children(|mut builder| {
                    // Note there is no need to specify the position for each grid item. Grid items that are
                    // not given an explicit position will be automatically positioned into the next available
                    // grid cell. The order in which this is performed can be controlled using the grid_auto_flow
                    // style property.

                    item_rect(&mut builder, Color::ORANGE);
                    item_rect(&mut builder, Color::BISQUE);
                    item_rect(&mut builder, Color::BLUE);
                    item_rect(&mut builder, Color::CRIMSON);

                    item_rect(&mut builder, Color::CYAN);
                    item_rect(&mut builder, Color::ORANGE_RED);
                    item_rect(&mut builder, Color::DARK_GREEN);
                    item_rect(&mut builder, Color::FUCHSIA);

                    item_rect(&mut builder, Color::GOLD);
                    item_rect(&mut builder, Color::ALICE_BLUE);
                    item_rect(&mut builder, Color::GOLD);
                    item_rect(&mut builder, Color::ANTIQUE_WHITE);

                    item_rect(&mut builder, Color::GOLD);
                    item_rect(&mut builder, Color::GOLD);
                    item_rect(&mut builder, Color::GOLD);
                    item_rect(&mut builder, Color::GOLD);
                });

            // Right side bar
            // builder.spawn(rect(Color::BLACK));
            builder
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        align_items: AlignItems::Start,
                        justify_items: JustifyItems::Center,
                        padding: UiRect::top(Val::Px(20.)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                })
                .with_children(|builder| {
                    builder.spawn(TextBundle::from_section(
                        "Sidebar",
                        TextStyle {
                            font,
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Footer / status bar
            builder.spawn(NodeBundle {
                style: Style {
                    /// Make this node span two grid column so that it takes up the entire top tow
                    grid_column: GridPlacement::span(2),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            });

            // Modal (uncomment to view)
            // builder.spawn(NodeBundle {
            //     style: Style {
            //         position_type: PositionType::Absolute,
            //         margin: UiRect {
            //             top: Val::Px(100.),
            //             bottom: Val::Auto,
            //             left: Val::Auto,
            //             right: Val::Auto,
            //         },
            //         size: Size {
            //             width: Val::Percent(60.),
            //             height: Val::Px(300.),
            //         },
            //         max_size: Size {
            //             width: Val::Px(600.),
            //             height: Val::Auto,
            //         },
            //         ..default()
            //     },
            //     background_color: BackgroundColor(Color::Rgba {
            //         red: 255.0,
            //         green: 255.0,
            //         blue: 255.0,
            //         alpha: 0.8,
            //     }),
            //     ..default()
            // });
        });
}

/// Create a coloured rectangle node. The node has size as it is assumed that it will be
/// spawned as a child of a Grid container with AlignItems::Stretch and JustifyItems::Stretch
/// which will allow it to take it's size from the size of the grid area it occupies.
fn item_rect(builder: &mut ChildBuilder, color: Color) {
    builder
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                padding: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                background_color: BackgroundColor(color),
                ..default()
            });
        });
}

fn spawn_nested_text_bundle(builder: &mut ChildBuilder, font: Handle<Font>, text: &str) {
    builder
        .spawn(NodeBundle {
            style: Style {
                min_size: Size::width(Val::Px(0.)),
                ..default()
            },
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
