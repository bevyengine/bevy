//! Simple example demonstrating linear gradients.

use bevy::{
    color::palettes::css::{BLUE, GREEN, INDIGO, LIME, ORANGE, RED, VIOLET, YELLOW},
    picking::hover::Hovered,
    prelude::*,
    ui::{ColorStop, Selected},
    ui_widgets::{ControlOrientation, ListBox, ListItem, Scrollbar, ScrollbarThumb, ValueChange},
};
use std::f32::consts::TAU;

#[derive(Component, Clone, Copy, Default, PartialEq)]
struct ColorSpaceOption(InterpolationColorSpace);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update, list_item_hovered_style))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Scenes associated with the color space listbox picker.
    // They are added to the correct parent below.
    let color_space_help_text_id = commands
        .spawn_scene(color_space_list_box_help_text_scene())
        .id();
    let color_space_list_id = commands.spawn_scene(color_space_list_box_scene()).id();

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(20),
            margin: UiRect::all(px(20)),
            ..Default::default()
        })
        .with_children(|commands| {
            for (b, stops) in [
                (
                    4.,
                    vec![
                        ColorStop::new(Color::WHITE, percent(15)),
                        ColorStop::new(Color::BLACK, percent(85)),
                    ],
                ),
                (4., vec![RED.into(), BLUE.into(), LIME.into()]),
                (
                    0.,
                    vec![
                        RED.into(),
                        ColorStop::new(RED, percent(100. / 7.)),
                        ColorStop::new(ORANGE, percent(100. / 7.)),
                        ColorStop::new(ORANGE, percent(200. / 7.)),
                        ColorStop::new(YELLOW, percent(200. / 7.)),
                        ColorStop::new(YELLOW, percent(300. / 7.)),
                        ColorStop::new(GREEN, percent(300. / 7.)),
                        ColorStop::new(GREEN, percent(400. / 7.)),
                        ColorStop::new(BLUE, percent(400. / 7.)),
                        ColorStop::new(BLUE, percent(500. / 7.)),
                        ColorStop::new(INDIGO, percent(500. / 7.)),
                        ColorStop::new(INDIGO, percent(600. / 7.)),
                        ColorStop::new(VIOLET, percent(600. / 7.)),
                        VIOLET.into(),
                    ],
                ),
            ] {
                commands.spawn(Node::default()).with_children(|commands| {
                    commands
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(5),
                            ..Default::default()
                        })
                        .with_children(|commands| {
                            for (w, h) in [(70., 70.), (35., 70.), (70., 35.)] {
                                commands
                                    .spawn(Node {
                                        column_gap: px(10),
                                        ..Default::default()
                                    })
                                    .with_children(|commands| {
                                        for angle in (0..8).map(|i| i as f32 * TAU / 8.) {
                                            commands.spawn((
                                                Node {
                                                    width: px(w),
                                                    height: px(h),
                                                    border: UiRect::all(px(b)),
                                                    border_radius: BorderRadius::all(px(20)),
                                                    ..default()
                                                },
                                                BackgroundGradient::from(LinearGradient {
                                                    angle,
                                                    stops: stops.clone(),
                                                    ..default()
                                                }),
                                                BorderGradient::from(LinearGradient {
                                                    angle: 3. * TAU / 8.,
                                                    stops: vec![
                                                        YELLOW.into(),
                                                        Color::WHITE.into(),
                                                        ORANGE.into(),
                                                    ],
                                                    ..default()
                                                }),
                                            ));
                                        }
                                    });
                            }
                        });

                    commands.spawn(Node::default()).with_children(|commands| {
                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: percent(100),
                                border: UiRect::all(px(b)),
                                margin: UiRect::left(px(20)),
                                border_radius: BorderRadius::all(px(20)),
                                ..default()
                            },
                            BackgroundGradient::from(LinearGradient {
                                angle: 0.,
                                stops: stops.clone(),
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));

                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: percent(100),
                                border: UiRect::all(px(b)),
                                margin: UiRect::left(px(20)),
                                border_radius: BorderRadius::all(px(20)),
                                ..default()
                            },
                            BackgroundGradient::from(RadialGradient {
                                stops: stops.clone(),
                                shape: RadialGradientShape::ClosestSide,
                                position: UiPosition::CENTER,
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));
                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: percent(100),
                                border: UiRect::all(px(b)),
                                margin: UiRect::left(px(20)),
                                border_radius: BorderRadius::all(px(20)),
                                ..default()
                            },
                            BackgroundGradient::from(ConicGradient {
                                start: 0.,
                                stops: stops
                                    .iter()
                                    .map(|stop| AngularColorStop::auto(stop.color))
                                    .collect(),
                                position: UiPosition::CENTER,
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));
                    });
                });
            }

            commands
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                })
                .add_child(color_space_list_id)
                .add_child(color_space_help_text_id);
        });
}

/// Directions shown to the user.
fn color_space_list_box_help_text_scene() -> impl Scene {
    bsn! {
        Node
        Children [
            Text::new("Click on a color space in the list box to change the example.")
        ]
    }
}

/// Returns the scene that powers the user-interactable listbox.
/// The user can update the color space used in the example by clicking on an item.
fn color_space_list_box_scene() -> impl Scene {
    bsn! {
        Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)],
        }
        Children [
            #ListContent
            ListBox
            Node {
                flex_direction: FlexDirection::Column,
                height: px(75)
                overflow: Overflow::scroll_y(),
            }
            ScrollPosition::default()
            BackgroundColor(Srgba::new(0.7, 0.7, 0.7, 1.0))
            on(on_value_change)
            Children [
                ListItem
                Selected
                Hovered::default()
                BackgroundColor(Color::BLACK)
                ColorSpaceOption(InterpolationColorSpace::Oklaba)
                Text::new(format!("{:?}", InterpolationColorSpace::Oklaba)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::Oklcha)
                Text::new(format!("{:?}", InterpolationColorSpace::Oklcha)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::OklchaLong)
                Text::new(format!("{:?}", InterpolationColorSpace::OklchaLong)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::Srgba)
                Text::new(format!("{:?}", InterpolationColorSpace::Srgba)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::LinearRgba)
                Text::new(format!("{:?}", InterpolationColorSpace::LinearRgba)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::Hsla)
                Text::new(format!("{:?}", InterpolationColorSpace::Hsla)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::HslaLong)
                Text::new(format!("{:?}", InterpolationColorSpace::HslaLong)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::Hsva)
                Text::new(format!("{:?}", InterpolationColorSpace::Hsva)),

                ListItem
                Hovered::default()
                ColorSpaceOption(InterpolationColorSpace::HsvaLong)
                Text::new(format!("{:?}", InterpolationColorSpace::HsvaLong)),
            ],

            // Scrollbar
            Node {
                min_width: px(12),
            }
            Scrollbar {
                orientation: ControlOrientation::Vertical,
                target: #ListContent,
                min_thumb_length: 8.0,
            }
            Children [
                BackgroundColor(Color::WHITE)
                BorderColor::all(Color::BLACK)
                ScrollbarThumb {
                    border_radius: BorderRadius::all(px(4)),
                    border: UiRect::all(px(1)),
                }
            ],
        ]
    }
}

/// Handles the value change of the listbox entity when a color space is selected.
fn on_value_change(
    event: On<ValueChange<Entity>>,
    color_space_selection_query: Query<&ColorSpaceOption>,
    children_query: Query<&Children>,
    mut gradients_query: Query<&mut BackgroundGradient>,
    mut commands: Commands,
) {
    let Ok(ColorSpaceOption(next_space)) = color_space_selection_query.get(event.value) else {
        return;
    };
    for mut gradients in gradients_query.iter_mut() {
        for gradient in gradients.0.iter_mut() {
            let space = match gradient {
                Gradient::Linear(linear_gradient) => &mut linear_gradient.color_space,
                Gradient::Radial(radial_gradient) => &mut radial_gradient.color_space,
                Gradient::Conic(conic_gradient) => &mut conic_gradient.color_space,
            };
            *space = *next_space;
        }
    }
    for child in children_query.iter_descendants(event.source) {
        let Ok(ColorSpaceOption(space)) = color_space_selection_query.get(child) else {
            return;
        };
        if space == next_space {
            commands.entity(child).insert(Selected);
            commands.entity(child).insert(BackgroundColor(Color::BLACK));
        } else {
            commands.entity(child).remove::<Selected>();
            commands.entity(child).remove::<BackgroundColor>();
        }
    }
}

#[derive(Component)]
struct AnimateMarker;

fn update(time: Res<Time>, mut query: Query<&mut BackgroundGradient, With<AnimateMarker>>) {
    for mut gradients in query.iter_mut() {
        for gradient in gradients.0.iter_mut() {
            if let Gradient::Linear(LinearGradient { angle, .. }) = gradient {
                *angle += 0.5 * time.delta_secs();
            }
        }
    }
}

/// A system that updates the styling of listboxes depending on their hover and selected states.
fn list_item_hovered_style(
    mut bg_q: Query<(Entity, &Hovered, Has<Selected>), (Changed<Hovered>, With<ListItem>)>,
    mut commands: Commands,
) {
    for (entity, hovered, selected) in bg_q.iter_mut() {
        if selected {
            continue;
        }
        if hovered.get() {
            commands
                .entity(entity)
                .insert(BackgroundColor(Color::BLACK));
        } else {
            commands.entity(entity).remove::<BackgroundColor>();
        }
    }
}
