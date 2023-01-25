//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [1200., 1000.].into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_startup_system(spawn_layout)
        .run();
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands
        // spawn the root panel
        .spawn(NodeBundle {
            style: Style {
                // Fill the entire window
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                // Wrap the child nodes into rows
                flex_wrap: FlexWrap::Wrap,
                // Stack the wrapped child nodes downwards from the top, without spacing between the rows.
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::CYAN),
            ..Default::default()
        })
        // spawn one child node for each combination of `AlignItems` and `JustifyContent`
        .with_children(|builder| {
            let alignments = [
                AlignItems::Baseline,
                AlignItems::Center,
                AlignItems::FlexEnd,
                AlignItems::FlexStart,
                AlignItems::Stretch,
            ];
            let justifications = [
                JustifyContent::Center,
                JustifyContent::FlexEnd,
                JustifyContent::FlexStart,
                JustifyContent::SpaceAround,
                JustifyContent::SpaceEvenly,
                JustifyContent::SpaceBetween,
            ];
            for align_items in alignments.into_iter() {
                for justify_content in justifications.into_iter() {
                    spawn_child_node(builder, &asset_server, align_items, justify_content);
                }
            }
        });
}

fn spawn_child_node(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    align_items: AlignItems,
    justify_content: JustifyContent,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items,
                justify_content,
                size: Size::new(Val::Px(190.), Val::Px(190.)),
                margin: UiRect::all(Val::Px(5.)),
                padding: UiRect::all(Val::Px(3.)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .with_children(|builder| {
            let labels = [
                (format!("{:?}", align_items), Color::MAROON, 0.),
                (format!("{:?}", justify_content), Color::DARK_GREEN, 3.),
            ];
            for (text, color, top_margin) in labels {
                // We nest the text within a parent node because margins and padding can't be directly applied to text nodes currently.
                builder
                    .spawn(NodeBundle {
                        style: Style {
                            margin: UiRect::top(Val::Px(top_margin)),
                            padding: UiRect::all(Val::Px(5.)),
                            ..Default::default()
                        },
                        background_color: BackgroundColor(color),
                        ..Default::default()
                    })
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            text,
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 24.0,
                                color: Color::ANTIQUE_WHITE,
                            },
                        ));
                    });
            }
        });
}
