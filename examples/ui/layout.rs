//! Example that creates a more complex UI layout and demonstrates how
//! the different JustifyContent and AlignItems variants compose to determine the positions of items within the layout.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 1200.,
                height: 1000.,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle {
        camera_2d: Camera2d {
            clear_color: bevy::core_pipeline::clear_color::ClearColorConfig::Custom(Color::BLACK),
        },
        ..Default::default()
    });
    commands
        .spawn(NodeBundle {
            // Root Panel, covers the entire screen
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::CYAN),
            ..Default::default()
        })
        .with_children(|builder| {
            let items = [
                AlignItems::Baseline,
                AlignItems::Center,
                AlignItems::FlexEnd,
                AlignItems::FlexStart,
                AlignItems::Stretch,
            ];
            let justs = [
                JustifyContent::Center,
                JustifyContent::FlexEnd,
                JustifyContent::FlexStart,
                JustifyContent::SpaceAround,
                JustifyContent::SpaceEvenly,
                JustifyContent::SpaceBetween,
            ];
            for align_items in items.into_iter() {
                for justify_content in justs.into_iter() {
                    spawn_panel(builder, &asset_server, align_items, justify_content);
                }
            }
        });
}

fn spawn_panel(
    parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
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
                overflow: Overflow::Hidden,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .with_children(|builder| {
            let items = [
                (format!("{:?}", align_items), Color::MAROON, Val::Px(0.)),
                (
                    format!("{:?}", justify_content),
                    Color::DARK_GREEN,
                    Val::Px(2.),
                ),
            ];
            for (text, color, top_margin) in items {
                builder
                    .spawn(NodeBundle {
                        style: Style {
                            margin: UiRect::top(top_margin),
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
