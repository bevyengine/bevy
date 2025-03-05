//! This example illustrates drag move and drag resize without window
//! decorations.
//!
//! When window decorations are not present, the user cannot drag a window by
//! its titlebar to change its position. The `start_drag_move()` function
//! permits a user to drag a window by left clicking anywhere in the window;
//! left click must be pressed and other constraints can be imposed. For
//! instance an application could require a user to hold down alt and left click
//! to drag a window.
//!
//! The `start_drag_resize()` function behaves similarly but permits a window to
//! be resized.
use bevy::{math::CompassOctant, prelude::*};

/// Determine what do on left click.
#[derive(Resource, Debug)]
enum LeftClickAction {
    /// Do nothing.
    Nothing,
    /// Move the window on left click.
    Move,
    /// Resize the window on left click.
    Resize,
}

/// What direction index should the window resize toward.
#[derive(Resource)]
struct ResizeDir(usize);

/// Directions that the drag resizes the window toward.
const DIRECTIONS: [CompassOctant; 8] = [
    CompassOctant::North,
    CompassOctant::NorthEast,
    CompassOctant::East,
    CompassOctant::SouthEast,
    CompassOctant::South,
    CompassOctant::SouthWest,
    CompassOctant::West,
    CompassOctant::NorthWest,
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                decorations: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ResizeDir(7))
        .insert_resource(LeftClickAction::Move)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, move_or_resize_windows))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera3d::default());

    // UI
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.75)),
            GlobalZIndex(i32::MAX),
        ))
        .with_children(|p| {
            p.spawn(Text::default()).with_children(|p| {
                p.spawn(TextSpan::new(
                    "Demonstrate drag move and drag resize without window decorations.\n\n",
                ));
                p.spawn(TextSpan::new("Controls:\n"));
                p.spawn(TextSpan::new("A - change left click action ["));
                p.spawn(TextSpan::new("Move"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new("S / D - change resize direction ["));
                p.spawn(TextSpan::new("NorthWest"));
                p.spawn(TextSpan::new("]\n"));
            });
        });
}

fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut action: ResMut<LeftClickAction>,
    mut dir: ResMut<ResizeDir>,
    example_text: Query<Entity, With<Text>>,
    mut writer: TextUiWriter,
) -> Result {
    use LeftClickAction::*;
    if input.just_pressed(KeyCode::KeyA) {
        *action = match *action {
            Move => Resize,
            Resize => Nothing,
            Nothing => Move,
        };
        *writer.text(example_text.single()?, 4) = format!("{:?}", *action);
    }

    if input.just_pressed(KeyCode::KeyS) {
        dir.0 = dir
            .0
            .checked_sub(1)
            .unwrap_or(DIRECTIONS.len().saturating_sub(1));
        *writer.text(example_text.single()?, 7) = format!("{:?}", DIRECTIONS[dir.0]);
    }

    if input.just_pressed(KeyCode::KeyD) {
        dir.0 = (dir.0 + 1) % DIRECTIONS.len();
        *writer.text(example_text.single()?, 7) = format!("{:?}", DIRECTIONS[dir.0]);
    }

    Ok(())
}

fn move_or_resize_windows(
    mut windows: Query<&mut Window>,
    action: Res<LeftClickAction>,
    input: Res<ButtonInput<MouseButton>>,
    dir: Res<ResizeDir>,
) {
    // Both `start_drag_move()` and `start_drag_resize()` must be called after a
    // left mouse button press as done here.
    //
    // winit 0.30.5 may panic when initiated without a left mouse button press.
    if input.just_pressed(MouseButton::Left) {
        for mut window in windows.iter_mut() {
            match *action {
                LeftClickAction::Nothing => (),
                LeftClickAction::Move => window.start_drag_move(),
                LeftClickAction::Resize => {
                    let d = DIRECTIONS[dir.0];
                    window.start_drag_resize(d);
                }
            }
        }
    }
}
