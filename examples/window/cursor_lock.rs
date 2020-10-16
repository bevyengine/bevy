use bevy::{
    prelude::*,
    window::{
        ChangeCursorLockState, ChangeCursorVisibility, CursorLockMode, CursorShowMode, WindowId,
    },
};

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Press G to grab the cursor".to_string(),
            ..Default::default()
        })
        .add_default_plugins()
        .add_system(grab_cursor_system.system())
        .run()
}

fn grab_cursor_system(
    input: Res<Input<KeyCode>>,
    mut lock_state: ResMut<Events<ChangeCursorLockState>>,
    mut show_state: ResMut<Events<ChangeCursorVisibility>>,
) {
    if input.just_pressed(KeyCode::G) {
        let id = WindowId::primary();

        lock_state.send(ChangeCursorLockState {
            id,
            mode: CursorLockMode::Locked,
        });

        show_state.send(ChangeCursorVisibility {
            id,
            mode: CursorShowMode::Hide,
        });
    }
}
