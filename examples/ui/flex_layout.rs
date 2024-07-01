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
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                // fill the entire window
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(MARGIN),
                row_gap: MARGIN,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .with_children(|builder| {
            // spawn the key
            builder
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                    ..Default::default()
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
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: MARGIN,
                        ..Default::default()
                    },
                    ..Default::default()
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
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.),
                                    height: Val::Percent(100.),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: MARGIN,
                                    ..Default::default()
                                },
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
    builder: &mut ChildBuilder,
    font: Handle<Font>,
    align_items: AlignItems,
    justify_content: JustifyContent,
) {
    builder
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items,
                justify_content,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            ..Default::default()
        })
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
                    UiRect::top(Val::Px(top_margin)),
                    &text,
                );
            }
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
                padding: UiRect::axes(Val::Px(5.), Val::Px(1.)),
                ..Default::default()
            },
            background_color: BackgroundColor(background_color),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font,
                    color: Color::BLACK,
                    ..default()
                },
            ));
        });
}
