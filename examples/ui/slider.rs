//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{prelude::*, ui::widget::Slider, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(update)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                flex_direction: FlexDirection::ColumnReverse,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(SliderBundle {
                    style: Style {
                        size: Size::new(Val::Px(200.), Val::Px(20.)),
                        margin: UiRect::bottom(Val::Px(25.)),
                        ..default()
                    },
                    background_color: Color::rgb(0.8, 0.8, 0.8).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(SliderHandleBundle {
                        style: Style {
                            size: Size::new(Val::Px(15.), Val::Px(20.)),
                            ..default()
                        },
                        ..default()
                    });
                });

            parent.spawn(TextBundle::from_section(
                "0",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ));
        });
}

fn update(slider_query: Query<&Slider>, mut text_query: Query<&mut Text>) {
    let slider = slider_query.single();
    let mut text = text_query.single_mut();

    text.sections[0].value = format!("{}", slider.get_value().round());
}
