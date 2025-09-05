//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::prelude::*;

const ALIGN_ITEMS_COLOR: Color = Color::srgb(1., 0.066, 0.349);
const JUSTIFY_CONTENT_COLOR: Color = Color::srgb(0.102, 0.522, 1.);
const MARGIN: Val = Val::Px(12.);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Flex Layout Example".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, spawn_layout)
        .run();
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                // fill the entire window
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(MARGIN),
                row_gap: MARGIN,
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|builder| {
            // spawn the key
            builder
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|builder| {
                    spawn_nested_text_bundle(
                        builder,
                        font.clone(),
                        ALIGN_ITEMS_COLOR,
                        UiRect::right(MARGIN),
                        "AlignItems",
                    );
                    spawn_nested_text_bundle(
                        builder,
                        font.clone(),
                        JUSTIFY_CONTENT_COLOR,
                        UiRect::default(),
                        "JustifyContent",
                    );
                });

            builder
                .spawn(Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: MARGIN,
                    ..default()
                })
                .with_children(|builder| {
                    // spawn one child node for each combination of `AlignItems` and `JustifyContent`
                    let justifications = [
                        JustifyContent::FlexStart,
                        JustifyContent::Center,
                        JustifyContent::FlexEnd,
                        JustifyContent::SpaceEvenly,
                        JustifyContent::SpaceAround,
                        JustifyContent::SpaceBetween,
                    ];
                    let alignments = [
                        AlignItems::Baseline,
                        AlignItems::FlexStart,
                        AlignItems::Center,
                        AlignItems::FlexEnd,
                        AlignItems::Stretch,
                    ];
                    for align_items in alignments {
                        builder
                            .spawn(Node {
                                width: percent(100),
                                height: percent(100),
                                flex_direction: FlexDirection::Row,
                                column_gap: MARGIN,
                                ..Default::default()
                            })
                            .with_children(|builder| {
                                for justify_content in justifications {
                                    spawn_child_node(
                                        builder,
                                        font.clone(),
                                        align_items,
                                        justify_content,
                                    );
                                }
                            });
                    }
                });
        });
}

fn spawn_child_node(
    builder: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    align_items: AlignItems,
    justify_content: JustifyContent,
) {
    builder
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items,
                justify_content,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_children(|builder| {
            let labels = [
                (format!("{align_items:?}"), ALIGN_ITEMS_COLOR, 0.),
                (format!("{justify_content:?}"), JUSTIFY_CONTENT_COLOR, 3.),
            ];
            for (text, color, top_margin) in labels {
                // We nest the text within a parent node because margins and padding can't be directly applied to text nodes currently.
                spawn_nested_text_bundle(
                    builder,
                    font.clone(),
                    color,
                    UiRect::top(px(top_margin)),
                    &text,
                );
            }
        });
}

fn spawn_nested_text_bundle(
    builder: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    background_color: Color,
    margin: UiRect,
    text: &str,
) {
    builder
        .spawn((
            Node {
                margin,
                padding: UiRect::axes(px(5), px(1)),
                ..default()
            },
            BackgroundColor(background_color),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text::new(text),
                TextFont { font, ..default() },
                TextColor::BLACK,
            ));
        });
}
