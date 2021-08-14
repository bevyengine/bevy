use bevy::{
    input::gamepad::{Gamepad, GamepadButton, GamepadEvent, GamepadEventType},
    prelude::*,
    utils::HashSet,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<GamepadLobby>()
        .add_system_to_stage(CoreStage::PreUpdate, connection_system)
        .add_system(gamepad_system)
        .run();
}

#[derive(Default)]
struct GamepadLobby {
    gamepads: HashSet<Gamepad>,
}

fn connection_system(
    mut lobby: ResMut<GamepadLobby>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.iter() {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                lobby.gamepads.insert(*gamepad);
                info!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                lobby.gamepads.remove(gamepad);
                info!("{:?} Disconnected", gamepad);
            }
            _ => (),
        }
    }
}

fn gamepad_system(
    lobby: Res<GamepadLobby>,
    button_inputs: Res<Input<GamepadButton>>,
    button_axes: Res<Axis<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
) {
    for gamepad in lobby.gamepads.iter().cloned() {
        if button_inputs.just_pressed(GamepadButton(gamepad, GamepadButtonType::South)) {
            info!("{:?} just pressed South", gamepad);
        } else if button_inputs.just_released(GamepadButton(gamepad, GamepadButtonType::South)) {
            info!("{:?} just released South", gamepad);
        }

        let right_trigger = button_axes
            .get(GamepadButton(gamepad, GamepadButtonType::RightTrigger2))
            .unwrap();
        if right_trigger.abs() > 0.01 {
            info!("{:?} RightTrigger2 value is {}", gamepad, right_trigger);
        }

        let left_stick_x = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::LeftStickX))
            .unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad, left_stick_x);
        }
    }
}
