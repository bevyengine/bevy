//! This example illustrates drag and drag resize without window decorations.
//!
//! When window decorations are not present, the user cannot drag the window.
//! The `start_drag_move()` function will permit the application to make the
//! window draggable. It does require that the left mouse button was pressed
//! when it is called.
use bevy::{math::CompassOctant, prelude::*};

/// Determine what do on left click.
#[derive(Resource, Debug)]
enum LeftClickAction {
    /// Do nothing.
    Nothing,
    /// Drag the window on left click.
    Drag,
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
        .insert_resource(LeftClickAction::Drag)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, move_windows))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera3d::default());

    // UI
    let style = TextStyle::default();
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                background_color: Color::BLACK.with_alpha(0.75).into(),
                ..default()
            },
            GlobalZIndex(i32::MAX),
        ))
        .with_children(|c| {
            c.spawn(TextBundle::from_sections([
                TextSection::new(
                    "Demonstrate drag and drag resize without window decorations.\n\nControls:\n",
                    style.clone(),
                ),
                TextSection::new("A - change left click action [", style.clone()),
                TextSection::new("Drag", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new("S / D - change resize direction [", style.clone()),
                TextSection::new("NorthWest", style.clone()),
                TextSection::new("]\n", style.clone()),
            ]));
        });
}

fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut action: ResMut<LeftClickAction>,
    mut dir: ResMut<ResizeDir>,
    mut example_text: Query<&mut Text>,
) {
    use LeftClickAction::*;
    if input.just_pressed(KeyCode::KeyA) {
        *action = match *action {
            Drag => Resize,
            Resize => Nothing,
            Nothing => Drag,
        };
        let mut example_text = example_text.single_mut();
        example_text.sections[2].value = format!("{:?}", *action);
    }

    if input.just_pressed(KeyCode::KeyS) {
        dir.0 = dir
            .0
            .checked_sub(1)
            .unwrap_or(DIRECTIONS.len().saturating_sub(1));
        let mut example_text = example_text.single_mut();
        example_text.sections[5].value = format!("{:?}", DIRECTIONS[dir.0]);
    }

    if input.just_pressed(KeyCode::KeyD) {
        dir.0 = (dir.0 + 1) % DIRECTIONS.len();
        let mut example_text = example_text.single_mut();
        example_text.sections[5].value = format!("{:?}", DIRECTIONS[dir.0]);
    }
}

fn move_windows(
    mut windows: Query<&mut Window>,
    action: Res<LeftClickAction>,
    input: Res<ButtonInput<MouseButton>>,
    dir: Res<ResizeDir>,
) {
    // Both `start_drag_move()` and `start_drag_resize()` must be called after a
    // left mouse button press.
    //
    // winit 0.30.5 may panic when initiated without a left mouse button press.
    if input.just_pressed(MouseButton::Left) {
        for mut window in windows.iter_mut() {
            match *action {
                LeftClickAction::Nothing => (),
                LeftClickAction::Drag => window.start_drag_move(),
                LeftClickAction::Resize => {
                    let d = DIRECTIONS[dir.0];
                    window.start_drag_resize(d);
                }
            }
        }
    }
}
