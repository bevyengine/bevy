use bevy::{
    prelude::*,
    window::ResizeDirection,
};

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash, States)]
enum LeftClickAction {
    Nothing,
    Drag,
    Resize,
}

#[derive(Resource, Default)]
struct ResizeDir(ResizeDirection);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                decorations: false,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<ResizeDir>()
        .insert_state(LeftClickAction::Drag)
        .add_systems(Startup, setup)
        .add_systems(Update, (change_state, move_windows))
        .run();
}

fn setup(mut commands: Commands) {

    // camera
    commands.spawn((
        Camera3d::default(),
        // Transform::from_xyz(-1.0, 1.0, 1.0).looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
        // CameraController::default(),
    ));

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
                TextSection::new("Controls:\n", style.clone()),
                TextSection::new("A - change left click action [", style.clone()),
                TextSection::new("Drag", style.clone()),
                TextSection::new("]\n", style.clone()),

                TextSection::new("D - change resize direction [", style.clone()),
                TextSection::new("NorthWest", style.clone()),
                TextSection::new("]\n", style.clone()),
            ]));
        });
}

fn change_state(input: Res<ButtonInput<KeyCode>>,
                state: Res<State<LeftClickAction>>,
                mut next_state: ResMut<NextState<LeftClickAction>>,
                mut example_text: Query<&mut Text>) {
    use LeftClickAction::*;
    if input.just_pressed(KeyCode::KeyA) {
        let s = match state.get() {
            Drag => Resize,
            Resize => Nothing,
            Nothing => Drag,
        };
        next_state.set(s);
        let mut example_text = example_text.single_mut();
        example_text.sections[2].value = format!("{:?}", s);
    }
}


fn move_windows(mut windows: Query<&mut Window>,
                state: Res<State<LeftClickAction>>,
                input: Res<ButtonInput<MouseButton>>,
                dir: Res<ResizeDir>) {
    if input.just_pressed(MouseButton::Left) {
        for mut window in windows.iter_mut() {
            match state.get() {
                LeftClickAction::Nothing => (),
                LeftClickAction::Drag => window.start_drag_move(),
                LeftClickAction::Resize => window.start_drag_resize(dir.0),
            }
        }
    }
}
