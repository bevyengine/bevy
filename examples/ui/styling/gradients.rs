//! Simple example demonstrating linear gradients.

use bevy::{
    color::palettes::css::{BLUE, GREEN, INDIGO, LIME, ORANGE, RED, VIOLET, YELLOW},
    prelude::*,
    ui::ColorStop,
    ui_widgets::{Activate, Button},
};
use std::f32::consts::TAU;

const COLOR_SPACES: [InterpolationColorSpace; 11] = [
    InterpolationColorSpace::Oklaba,
    InterpolationColorSpace::Oklcha,
    InterpolationColorSpace::OklchaLong,
    InterpolationColorSpace::Srgba,
    InterpolationColorSpace::LinearRgba,
    InterpolationColorSpace::Hsla,
    InterpolationColorSpace::HslaLong,
    InterpolationColorSpace::Hsva,
    InterpolationColorSpace::HsvaLong,
    InterpolationColorSpace::Okhsla,
    InterpolationColorSpace::OkhslaLong,
];

/// Marker component for the previous button
#[derive(Component, Clone, Default)]
struct PreviousButton;

/// Marker component for the next button
#[derive(Component, Clone, Default)]
struct NextButton;

/// Marker component for the current color space label
#[derive(Component, Clone, Default)]
struct CurrentColorSpaceLabel;

/// Resource that holds the current settings for the app
#[derive(Resource, Default)]
struct AppSettings {
    /// The currently shown color space as an index into `COLOR_SPACES`
    color_space_current_index: usize,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AppSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_observer(on_activate_change_space)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let buttons_id = commands.spawn_scene(buttons_scene()).id();

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
        })
        .add_child(buttons_id);
}

/// Scene of the current color space, the previous button, and the next button.
/// The user can cycle through the color spaces by clicking the buttons.
fn buttons_scene() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(20),
        }
        Children [
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            }
            Children [
                CurrentColorSpaceLabel
                template(|ctx| {
                    let current_index = ctx.resource::<AppSettings>().color_space_current_index;
                    Ok(Text(format!("Current Space\n{:?}", COLOR_SPACES[current_index])))
                })
            ]
            --
            PreviousButton
            @button_node_scene("Previous")
            --
            NextButton
            @button_node_scene("Next")
        ]
    }
}

/// Base Button node scene
fn button_node_scene(caption: &'static str) -> impl Scene {
    bsn! {
        Button
        Node {
            border: UiRect::all(px(2)),
            padding: UiRect::axes(px(8), px(4)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
        }
        BorderColor::all(Color::WHITE)
        BackgroundColor(Color::BLACK)
        on(|event: On<PointerOver>, mut border_query: Query<&mut BorderColor, With<Button>>| {
            *border_query.get_mut(event.entity).unwrap() = BorderColor::all(RED);
        })
        on(|event: On<PointerOut>, mut border_query: Query<&mut BorderColor, With<Button>>| {
            *border_query.get_mut(event.entity).unwrap() = BorderColor::all(Color::WHITE);
        })
        Children [
            Text(caption)
        ]
    }
}

/// Observer that handles a button press from the previous/next buttons.
fn on_activate_change_space(
    event: On<Activate>,
    mut app_settings: ResMut<AppSettings>,
    button_type_q: Query<(Has<PreviousButton>, Has<NextButton>), With<Button>>,
    mut gradients_query: Query<&mut BackgroundGradient>,
    mut label_q: Query<&mut Text, With<CurrentColorSpaceLabel>>,
) {
    let Ok((has_previous, has_next)) = button_type_q.get(event.entity) else {
        return;
    };
    let next_index = match (has_previous, has_next) {
        (true, true) | (false, false) => return,
        (true, false) => {
            if app_settings.color_space_current_index == 0 {
                COLOR_SPACES.len() - 1
            } else {
                app_settings.color_space_current_index - 1
            }
        }
        (false, true) => (app_settings.color_space_current_index + 1) % COLOR_SPACES.len(),
    };
    app_settings.color_space_current_index = next_index;

    // Set the current space label and update the visuals.
    let next_space = COLOR_SPACES[app_settings.color_space_current_index];
    for mut label in label_q.iter_mut() {
        label.0 = format!("Current Space\n{next_space:?}");
    }
    for mut gradients in gradients_query.iter_mut() {
        for gradient in gradients.0.iter_mut() {
            let space = match gradient {
                Gradient::Linear(linear_gradient) => &mut linear_gradient.color_space,
                Gradient::Radial(radial_gradient) => &mut radial_gradient.color_space,
                Gradient::Conic(conic_gradient) => &mut conic_gradient.color_space,
            };
            *space = next_space;
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
