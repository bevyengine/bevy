//! This example illustrates how to create a slider and display its value on a text node.

use bevy::{
    prelude::*,
    ui::widget::{Slider, SliderHandle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_slider::<RegularSlider, RegularSliderOutput>)
        .add_system(update_slider::<SteppedSlider, SteppedSliderOutput>)
        .add_system(update_slider_handle_color)
        .run();
}

// Marker components
#[derive(Component)]
struct RegularSlider;

#[derive(Component)]
struct SteppedSlider;

#[derive(Component)]
struct RegularSliderOutput;

#[derive(Component)]
struct SteppedSliderOutput;

const DEFAULT_HANDLE_COLOR: Color = Color::rgb(1., 1., 1.);
const DRAGGED_HANDLE_COLOR: Color = Color::rgb(0.95, 0.95, 0.95);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(25.)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Adding the slider
                    parent
                        .spawn(SliderBundle {
                            style: Style {
                                size: Size::new(Val::Px(200.), Val::Px(20.)),
                                margin: UiRect::bottom(Val::Px(15.)),
                                ..default()
                            },
                            background_color: Color::rgb(0.8, 0.8, 0.8).into(),
                            ..default()
                        })
                        .insert(RegularSlider)
                        .with_children(|parent| {
                            // Adding the slider handle
                            parent.spawn(SliderHandleBundle {
                                style: Style {
                                    size: Size::new(Val::Px(15.), Val::Px(20.)),
                                    ..default()
                                },
                                background_color: DEFAULT_HANDLE_COLOR.into(),
                                ..default()
                            });
                        });

                    parent
                        .spawn(TextBundle::from_section(
                            "0",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ))
                        .insert(RegularSliderOutput);
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Adding the stepped slider
                    parent
                        .spawn(SliderBundle {
                            style: Style {
                                size: Size::new(Val::Px(200.), Val::Px(20.)),
                                margin: UiRect::bottom(Val::Px(15.)),
                                ..default()
                            },
                            background_color: Color::rgb(0.8, 0.8, 0.8).into(),
                            slider: Slider::new(50., 150.).with_step(10.),
                            ..default()
                        })
                        .insert(SteppedSlider)
                        .with_children(|parent| {
                            // Adding the slider handle
                            parent.spawn(SliderHandleBundle {
                                style: Style {
                                    size: Size::new(Val::Px(15.), Val::Px(20.)),
                                    ..default()
                                },
                                background_color: DEFAULT_HANDLE_COLOR.into(),
                                ..default()
                            });
                        });

                    parent
                        .spawn(TextBundle::from_section(
                            "0",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ))
                        .insert(SteppedSliderOutput);
                });
        });
}

fn update_slider<SliderMarker: Component, OutputMarker: Component>(
    slider_query: Query<&Slider, With<SliderMarker>>,
    mut text_query: Query<&mut Text, With<OutputMarker>>,
) {
    let slider = slider_query.single();
    let mut text = text_query.single_mut();

    text.sections[0].value = format!("{}", slider.value().round());
}

fn update_slider_handle_color(
    slider_query: Query<(&Interaction, &Children)>,
    mut slider_handle_query: Query<&mut BackgroundColor, With<SliderHandle>>,
) {
    for (slider_interaction, slider_children) in slider_query.iter() {
        for child in slider_children.iter() {
            if let Ok(mut handle_color) = slider_handle_query.get_mut(*child) {
                handle_color.0 = if *slider_interaction == Interaction::Clicked {
                    DRAGGED_HANDLE_COLOR
                } else {
                    DEFAULT_HANDLE_COLOR
                };
            }
        }
    }
}
