//! Demonstrates how CSS Grid layout can be used to lay items out in a 2D grid
use bevy::{color::palettes::css::*, prelude::*};

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
        .add_systems(Startup, spawn_layout)
        .run();
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2d);

    // Top-level grid (app frame)
    commands
        .spawn((
            Node {
                // Use the CSS Grid algorithm for laying out this node
                display: Display::Grid,
                // Make node fill the entirety of its parent (in this case the window)
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                // Set the grid to have 2 columns with sizes [min-content, minmax(0, 1fr)]
                //   - The first column will size to the size of its contents
                //   - The second column will take up the remaining available space
                grid_template_columns: vec![GridTrack::min_content(), GridTrack::flex(1.0)],
                // Set the grid to have 3 rows with sizes [auto, minmax(0, 1fr), 20px]
                //  - The first row will size to the size of its contents
                //  - The second row take up remaining available space (after rows 1 and 3 have both been sized)
                //  - The third row will be exactly 20px high
                grid_template_rows: vec![
                    GridTrack::auto(),
                    GridTrack::flex(1.0),
                    GridTrack::px(20.),
                ],
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .with_children(|builder| {
            // Header
            builder
                .spawn(
                    Node {
                        display: Display::Grid,
                        // Make this node span two grid columns so that it takes up the entire top tow
                        grid_column: GridPlacement::span(2),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                )
                .with_children(|builder| {
                    spawn_nested_text_bundle(builder, font.clone(), "Bevy CSS Grid Layout Example");
                });

            // Main content grid (auto placed in row 2, column 1)
            builder
                .spawn((
                    Node {
                        // Make the height of the node fill its parent
                        height: Val::Percent(100.0),
                        // Make the grid have a 1:1 aspect ratio meaning it will scale as an exact square
                        // As the height is set explicitly, this means the width will adjust to match the height
                        aspect_ratio: Some(1.0),
                        // Use grid layout for this node
                        display: Display::Grid,
                        // Add 24px of padding around the grid
                        padding: UiRect::all(Val::Px(24.0)),
                        // Set the grid to have 4 columns all with sizes minmax(0, 1fr)
                        // This creates 4 exactly evenly sized columns
                        grid_template_columns: RepeatedGridTrack::flex(4, 1.0),
                        // Set the grid to have 4 rows all with sizes minmax(0, 1fr)
                        // This creates 4 exactly evenly sized rows
                        grid_template_rows: RepeatedGridTrack::flex(4, 1.0),
                        // Set a 12px gap/gutter between rows and columns
                        row_gap: Val::Px(12.0),
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                ))
                .with_children(|builder| {
                    // Note there is no need to specify the position for each grid item. Grid items that are
                    // not given an explicit position will be automatically positioned into the next available
                    // grid cell. The order in which this is performed can be controlled using the grid_auto_flow
                    // style property.

                    item_rect(builder, ORANGE);
                    item_rect(builder, BISQUE);
                    item_rect(builder, BLUE);
                    item_rect(builder, CRIMSON);
                    item_rect(builder, AQUA);
                    item_rect(builder, ORANGE_RED);
                    item_rect(builder, DARK_GREEN);
                    item_rect(builder, FUCHSIA);
                    item_rect(builder, TEAL);
                    item_rect(builder, ALICE_BLUE);
                    item_rect(builder, CRIMSON);
                    item_rect(builder, ANTIQUE_WHITE);
                    item_rect(builder, YELLOW);
                    item_rect(builder, DEEP_PINK);
                    item_rect(builder, YELLOW_GREEN);
                    item_rect(builder, SALMON);
                });

            // Right side bar (auto placed in row 2, column 2)
            builder
                .spawn((
                    Node {
                        display: Display::Grid,
                        // Align content towards the start (top) in the vertical axis
                        align_items: AlignItems::Start,
                        // Align content towards the center in the horizontal axis
                        justify_items: JustifyItems::Center,
                        // Add 10px padding
                        padding: UiRect::all(Val::Px(10.)),
                        // Add an fr track to take up all the available space at the bottom of the column so that the text nodes
                        // can be top-aligned. Normally you'd use flexbox for this, but this is the CSS Grid example so we're using grid.
                        grid_template_rows: vec![GridTrack::auto(), GridTrack::auto(), GridTrack::fr(1.0)],
                        // Add a 10px gap between rows
                        row_gap: Val::Px(10.),
                        ..default()
                    },
                    BackgroundColor(BLACK.into()),
                ))
                .with_children(|builder| {
                    builder.spawn((Text::new("Sidebar"),
                        TextFont {
                            font: font.clone(),
                            ..default()
                        },
                    ));
                    builder.spawn((Text::new("A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely. A paragraph of text which ought to wrap nicely."),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                    ));
                    builder.spawn(Node::default());
                });

            // Footer / status bar
            builder.spawn((
                Node {
                    // Make this node span two grid column so that it takes up the entire bottom row
                    grid_column: GridPlacement::span(2),
                    ..default()
                },
                BackgroundColor(WHITE.into()),
            ));

            // Modal (absolutely positioned on top of content - currently hidden: to view it, change its visibility)
            builder.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    margin: UiRect {
                        top: Val::Px(100.),
                        bottom: Val::Auto,
                        left: Val::Auto,
                        right: Val::Auto,
                    },
                    width: Val::Percent(60.),
                    height: Val::Px(300.),
                    max_width: Val::Px(600.),
                    ..default()
                },
                Visibility::Hidden,
                BackgroundColor(Color::WHITE.with_alpha(0.8)),
            ));
        });
}

/// Create a colored rectangle node. The node has size as it is assumed that it will be
/// spawned as a child of a Grid container with `AlignItems::Stretch` and `JustifyItems::Stretch`
/// which will allow it to take its size from the size of the grid area it occupies.
fn item_rect(builder: &mut ChildSpawnerCommands, color: Srgba) {
    builder
        .spawn((
            Node {
                display: Display::Grid,
                padding: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(BLACK.into()),
        ))
        .with_children(|builder| {
            builder.spawn((Node::default(), BackgroundColor(color.into())));
        });
}

fn spawn_nested_text_bundle(builder: &mut ChildSpawnerCommands, font: Handle<Font>, text: &str) {
    builder.spawn((
        Text::new(text),
        TextFont { font, ..default() },
        TextColor::BLACK,
    ));
}
