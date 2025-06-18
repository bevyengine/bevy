//! This example exhibits different available modes of constructing cubic Bezier curves.

use bevy::{
    app::{App, Startup, Update},
    color::*,
    ecs::system::Commands,
    gizmos::gizmos::Gizmos,
    input::{mouse::MouseButtonInput, ButtonState},
    math::{cubic_splines::*, vec2},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_keypress,
                handle_mouse_move,
                handle_mouse_press,
                draw_edit_move,
                update_curve,
                update_spline_mode_text,
                update_cycling_mode_text,
                draw_curve,
                draw_control_points,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands) {
    // Initialize the modes with their defaults:
    let spline_mode = SplineMode::default();
    commands.insert_resource(spline_mode);
    let cycling_mode = CyclingMode::default();
    commands.insert_resource(cycling_mode);

    // Starting data for [`ControlPoints`]:
    let default_points = vec![
        vec2(-500., -200.),
        vec2(-250., 250.),
        vec2(250., 250.),
        vec2(500., -200.),
    ];

    let default_tangents = vec![
        vec2(0., 200.),
        vec2(200., 0.),
        vec2(0., -200.),
        vec2(-200., 0.),
    ];

    let default_control_data = ControlPoints {
        points_and_tangents: default_points.into_iter().zip(default_tangents).collect(),
    };

    let curve = form_curve(&default_control_data, spline_mode, cycling_mode);
    commands.insert_resource(curve);
    commands.insert_resource(default_control_data);

    // Mouse tracking information:
    commands.insert_resource(MousePosition::default());
    commands.insert_resource(MouseEditMove::default());

    commands.spawn(Camera2d);

    // The instructions and modes are rendered on the left-hand side in a column.
    let instructions_text = "Click and drag to add control points and their tangents\n\
        R: Remove the last control point\n\
        S: Cycle the spline construction being used\n\
        C: Toggle cyclic curve construction";
    let spline_mode_text = format!("Spline: {spline_mode}");
    let cycling_mode_text = format!("{cycling_mode}");
    let style = TextFont::default();

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((Text::new(instructions_text), style.clone()));
            parent.spawn((SplineModeText, Text(spline_mode_text), style.clone()));
            parent.spawn((CyclingModeText, Text(cycling_mode_text), style.clone()));
        });
}

// -----------------------------------
// Curve-related Resources and Systems
// -----------------------------------

/// The current spline mode, which determines the spline method used in conjunction with the
/// control points.
#[derive(Clone, Copy, Resource, Default)]
enum SplineMode {
    #[default]
    Hermite,
    Cardinal,
    B,
}

impl std::fmt::Display for SplineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplineMode::Hermite => f.write_str("Hermite"),
            SplineMode::Cardinal => f.write_str("Cardinal"),
            SplineMode::B => f.write_str("B"),
        }
    }
}

/// The current cycling mode, which determines whether the control points should be interpolated
/// cyclically (to make a loop).
#[derive(Clone, Copy, Resource, Default)]
enum CyclingMode {
    #[default]
    NotCyclic,
    Cyclic,
}

impl std::fmt::Display for CyclingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CyclingMode::NotCyclic => f.write_str("Not Cyclic"),
            CyclingMode::Cyclic => f.write_str("Cyclic"),
        }
    }
}

/// The curve presently being displayed. This is optional because there may not be enough control
/// points to actually generate a curve.
#[derive(Clone, Default, Resource)]
struct Curve(Option<CubicCurve<Vec2>>);

/// The control points used to generate a curve. The tangent components are only used in the case of
/// Hermite interpolation.
#[derive(Clone, Resource)]
struct ControlPoints {
    points_and_tangents: Vec<(Vec2, Vec2)>,
}

/// This system is responsible for updating the [`Curve`] when the [control points] or active modes
/// change.
///
/// [control points]: ControlPoints
fn update_curve(
    control_points: Res<ControlPoints>,
    spline_mode: Res<SplineMode>,
    cycling_mode: Res<CyclingMode>,
    mut curve: ResMut<Curve>,
) {
    if !control_points.is_changed() && !spline_mode.is_changed() && !cycling_mode.is_changed() {
        return;
    }

    *curve = form_curve(&control_points, *spline_mode, *cycling_mode);
}

/// This system uses gizmos to draw the current [`Curve`] by breaking it up into a large number
/// of line segments.
fn draw_curve(curve: Res<Curve>, mut gizmos: Gizmos) {
    let Some(ref curve) = curve.0 else {
        return;
    };
    // Scale resolution with curve length so it doesn't degrade as the length increases.
    let resolution = 100 * curve.segments().len();
    gizmos.linestrip(
        curve.iter_positions(resolution).map(|pt| pt.extend(0.0)),
        Color::srgb(1.0, 1.0, 1.0),
    );
}

/// This system uses gizmos to draw the current [control points] as circles, displaying their
/// tangent vectors as arrows in the case of a Hermite spline.
///
/// [control points]: ControlPoints
fn draw_control_points(
    control_points: Res<ControlPoints>,
    spline_mode: Res<SplineMode>,
    mut gizmos: Gizmos,
) {
    for &(point, tangent) in &control_points.points_and_tangents {
        gizmos.circle_2d(point, 10.0, Color::srgb(0.0, 1.0, 0.0));

        if matches!(*spline_mode, SplineMode::Hermite) {
            gizmos.arrow_2d(point, point + tangent, Color::srgb(1.0, 0.0, 0.0));
        }
    }
}

/// Helper function for generating a [`Curve`] from [control points] and selected modes.
///
/// [control points]: ControlPoints
fn form_curve(
    control_points: &ControlPoints,
    spline_mode: SplineMode,
    cycling_mode: CyclingMode,
) -> Curve {
    let (points, tangents): (Vec<_>, Vec<_>) =
        control_points.points_and_tangents.iter().copied().unzip();

    match spline_mode {
        SplineMode::Hermite => {
            let spline = CubicHermite::new(points, tangents);
            Curve(match cycling_mode {
                CyclingMode::NotCyclic => spline.to_curve().ok(),
                CyclingMode::Cyclic => spline.to_curve_cyclic().ok(),
            })
        }
        SplineMode::Cardinal => {
            let spline = CubicCardinalSpline::new_catmull_rom(points);
            Curve(match cycling_mode {
                CyclingMode::NotCyclic => spline.to_curve().ok(),
                CyclingMode::Cyclic => spline.to_curve_cyclic().ok(),
            })
        }
        SplineMode::B => {
            let spline = CubicBSpline::new(points);
            Curve(match cycling_mode {
                CyclingMode::NotCyclic => spline.to_curve().ok(),
                CyclingMode::Cyclic => spline.to_curve_cyclic().ok(),
            })
        }
    }
}

// --------------------
// Text-related Components and Systems
// --------------------

/// Marker component for the text node that displays the current [`SplineMode`].
#[derive(Component)]
struct SplineModeText;

/// Marker component for the text node that displays the current [`CyclingMode`].
#[derive(Component)]
struct CyclingModeText;

fn update_spline_mode_text(
    spline_mode: Res<SplineMode>,
    mut spline_mode_text: Query<&mut Text, With<SplineModeText>>,
) {
    if !spline_mode.is_changed() {
        return;
    }

    let new_text = format!("Spline: {}", *spline_mode);

    for mut spline_mode_text in spline_mode_text.iter_mut() {
        (**spline_mode_text).clone_from(&new_text);
    }
}

fn update_cycling_mode_text(
    cycling_mode: Res<CyclingMode>,
    mut cycling_mode_text: Query<&mut Text, With<CyclingModeText>>,
) {
    if !cycling_mode.is_changed() {
        return;
    }

    let new_text = format!("{}", *cycling_mode);

    for mut cycling_mode_text in cycling_mode_text.iter_mut() {
        (**cycling_mode_text).clone_from(&new_text);
    }
}

// -----------------------------------
// Input-related Resources and Systems
// -----------------------------------

/// A small state machine which tracks a click-and-drag motion used to create new control points.
///
/// When the user is not doing a click-and-drag motion, the `start` field is `None`. When the user
/// presses the left mouse button, the location of that press is temporarily stored in the field.
#[derive(Clone, Default, Resource)]
struct MouseEditMove {
    start: Option<Vec2>,
}

/// The current mouse position, if known.
#[derive(Clone, Default, Resource)]
struct MousePosition(Option<Vec2>);

/// Update the current cursor position and track it in the [`MousePosition`] resource.
fn handle_mouse_move(
    mut cursor_events: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(cursor_event) = cursor_events.read().last() {
        mouse_position.0 = Some(cursor_event.position);
    }
}

/// This system handles updating the [`MouseEditMove`] resource, orchestrating the logical part
/// of the click-and-drag motion which actually creates new control points.
fn handle_mouse_press(
    mut button_events: EventReader<MouseButtonInput>,
    mouse_position: Res<MousePosition>,
    mut edit_move: ResMut<MouseEditMove>,
    mut control_points: ResMut<ControlPoints>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let Some(mouse_pos) = mouse_position.0 else {
        return;
    };

    // Handle click and drag behavior
    for button_event in button_events.read() {
        if button_event.button != MouseButton::Left {
            continue;
        }

        match button_event.state {
            ButtonState::Pressed => {
                if edit_move.start.is_some() {
                    // If the edit move already has a start, press event should do nothing.
                    continue;
                }
                // This press represents the start of the edit move.
                edit_move.start = Some(mouse_pos);
            }

            ButtonState::Released => {
                // Release is only meaningful if we started an edit move.
                let Some(start) = edit_move.start else {
                    continue;
                };

                let (camera, camera_transform) = *camera;

                // Convert the starting point and end point (current mouse pos) into world coords:
                let Ok(point) = camera.viewport_to_world_2d(camera_transform, start) else {
                    continue;
                };
                let Ok(end_point) = camera.viewport_to_world_2d(camera_transform, mouse_pos) else {
                    continue;
                };
                let tangent = end_point - point;

                // The start of the click-and-drag motion represents the point to add,
                // while the difference with the current position represents the tangent.
                control_points.points_and_tangents.push((point, tangent));

                // Reset the edit move since we've consumed it.
                edit_move.start = None;
            }
        }
    }
}

/// This system handles drawing the "preview" control point based on the state of [`MouseEditMove`].
fn draw_edit_move(
    edit_move: Res<MouseEditMove>,
    mouse_position: Res<MousePosition>,
    mut gizmos: Gizmos,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let Some(start) = edit_move.start else {
        return;
    };
    let Some(mouse_pos) = mouse_position.0 else {
        return;
    };

    let (camera, camera_transform) = *camera;

    // Resources store data in viewport coordinates, so we need to convert to world coordinates
    // to display them:
    let Ok(start) = camera.viewport_to_world_2d(camera_transform, start) else {
        return;
    };
    let Ok(end) = camera.viewport_to_world_2d(camera_transform, mouse_pos) else {
        return;
    };

    gizmos.circle_2d(start, 10.0, Color::srgb(0.0, 1.0, 0.7));
    gizmos.circle_2d(start, 7.0, Color::srgb(0.0, 1.0, 0.7));
    gizmos.arrow_2d(start, end, Color::srgb(1.0, 0.0, 0.7));
}

/// This system handles all keyboard commands.
fn handle_keypress(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut spline_mode: ResMut<SplineMode>,
    mut cycling_mode: ResMut<CyclingMode>,
    mut control_points: ResMut<ControlPoints>,
) {
    // S => change spline mode
    if keyboard.just_pressed(KeyCode::KeyS) {
        *spline_mode = match *spline_mode {
            SplineMode::Hermite => SplineMode::Cardinal,
            SplineMode::Cardinal => SplineMode::B,
            SplineMode::B => SplineMode::Hermite,
        }
    }

    // C => change cycling mode
    if keyboard.just_pressed(KeyCode::KeyC) {
        *cycling_mode = match *cycling_mode {
            CyclingMode::NotCyclic => CyclingMode::Cyclic,
            CyclingMode::Cyclic => CyclingMode::NotCyclic,
        }
    }

    // R => remove last control point
    if keyboard.just_pressed(KeyCode::KeyR) {
        control_points.points_and_tangents.pop();
    }
}
