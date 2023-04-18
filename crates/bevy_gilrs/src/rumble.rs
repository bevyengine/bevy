//! Handle user specified rumble request events.
use bevy_ecs::{
    prelude::{EventReader, Res},
    system::NonSendMut,
};
use bevy_input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy_log::{debug, warn};
use bevy_time::Time;
use bevy_utils::HashMap;
use gilrs::{
    ff::{self, BaseEffect, BaseEffectType},
    GamepadId, Gilrs,
};

use crate::converter::convert_gamepad_id;

struct RunningRumble {
    deadline: f32,
    // We use `effect.drop()` to interact with this, but rustc can't know
    // gilrs uses Drop as an API feature.
    #[allow(dead_code)]
    effect: ff::Effect,
}

enum RumbleError {
    GamepadNotFound,
    GilrsError(ff::Error),
}
impl From<ff::Error> for RumbleError {
    fn from(err: ff::Error) -> Self {
        RumbleError::GilrsError(err)
    }
}

#[derive(Default)]
pub(crate) struct RumblesManager {
    rumbles: HashMap<GamepadId, Vec<RunningRumble>>,
}

fn to_gilrs_magnitude(ratio: f32) -> u16 {
    (ratio * u16::MAX as f32) as u16
}

fn get_base_effects(
    GamepadRumbleIntensity { weak, strong }: GamepadRumbleIntensity,
) -> Vec<ff::BaseEffect> {
    let mut effects = Vec::new();
    if strong > 0. {
        effects.push(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: to_gilrs_magnitude(strong),
            },
            ..Default::default()
        });
    }
    if weak > 0. {
        effects.push(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: to_gilrs_magnitude(weak),
            },
            ..Default::default()
        });
    }
    effects
}

fn handle_rumble_request(
    manager: &mut RumblesManager,
    gilrs: &mut Gilrs,
    rumble: GamepadRumbleRequest,
    current_time: f32,
) -> Result<(), RumbleError> {
    let gamepad = match rumble {
        GamepadRumbleRequest::Add { gamepad, .. } => gamepad,
        GamepadRumbleRequest::Stop { gamepad } => gamepad,
    };
    let (gamepad_id, _) = gilrs
        .gamepads()
        .find(|(pad_id, _)| convert_gamepad_id(*pad_id) == gamepad)
        .ok_or(RumbleError::GamepadNotFound)?;

    match rumble {
        GamepadRumbleRequest::Stop { .. } => {
            // `ff::Effect` uses RAII, dropping = deactivating
            manager.rumbles.remove(&gamepad_id);
        }
        GamepadRumbleRequest::Add {
            duration,
            intensity,
            ..
        } => {
            let deadline = current_time + duration.as_secs_f32();

            let mut effect_builder = ff::EffectBuilder::new();

            for effect in get_base_effects(intensity) {
                effect_builder.add_effect(effect);
            }

            let effect = effect_builder.gamepads(&[gamepad_id]).finish(gilrs)?;
            effect.play()?;

            let gamepad_rumbles = manager.rumbles.entry(gamepad_id).or_default();
            gamepad_rumbles.push(RunningRumble { deadline, effect });
        }
    }

    Ok(())
}
pub(crate) fn play_gilrs_rumble(
    time: Res<Time>,
    mut gilrs: NonSendMut<Gilrs>,
    mut requests: EventReader<GamepadRumbleRequest>,
    mut manager: NonSendMut<RumblesManager>,
) {
    let current_time = time.elapsed_seconds();
    // Remove outdated rumble effects.
    for (_gamepad, rumbles) in manager.rumbles.iter_mut() {
        // `ff::Effect` uses RAII, dropping = deactivating
        rumbles.retain(|RunningRumble { deadline, .. }| *deadline >= current_time);
    }
    manager
        .rumbles
        .retain(|_gamepad, rumbles| !rumbles.is_empty());

    // Add new effects.
    for rumble in requests.iter().cloned() {
        let gamepad = match rumble {
            GamepadRumbleRequest::Add { gamepad, .. } => gamepad,
            GamepadRumbleRequest::Stop { gamepad } => gamepad,
        };
        match handle_rumble_request(&mut manager, &mut gilrs, rumble, current_time) {
            Ok(()) => {}
            Err(RumbleError::GilrsError(err)) => {
                debug!(
                    "Tried to handle rumble request for {gamepad:?} but an error occurred: {err}"
                );
            }
            Err(RumbleError::GamepadNotFound) => {
                warn!("Tried to handle rumble request {gamepad:?} but it doesn't exist!");
            }
        };
    }
}
