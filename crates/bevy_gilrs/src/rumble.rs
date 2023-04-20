//! Handle user specified rumble request events.
use bevy_ecs::{
    prelude::{EventReader, Res},
    system::NonSendMut,
};
use bevy_input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy_log::{debug, warn};
use bevy_time::Time;
use bevy_utils::{Duration, HashMap};
use gilrs::{
    ff::{self, BaseEffect, BaseEffectType, Repeat, Replay},
    GamepadId, Gilrs,
};
use thiserror::Error;

use crate::converter::convert_gamepad_id;

/// A rumble effect that is currently in effect.
struct RunningRumble {
    /// Duration from app startup when this effect will be finished
    deadline: Duration,
    /// A ref-counted handle to the specific force-feedback effect
    ///
    /// Dropping it will cause the effect to stop
    #[allow(dead_code)]
    effect: ff::Effect,
}

#[derive(Error, Debug)]
enum RumbleError {
    #[error("gamepad not found")]
    GamepadNotFound,
    #[error("gilrs error while rumbling gamepad: {0}")]
    GilrsError(#[from] ff::Error),
}

#[derive(Default)]
pub(crate) struct RumblesManager {
    /// If multiple rumbles are running at the same time, their resulting rumble
    /// will be the saturated sum of their strengths up until [`u16::MAX`]
    rumbles: HashMap<GamepadId, Vec<RunningRumble>>,
}

fn to_gilrs_magnitude(ratio: f32) -> u16 {
    (ratio * u16::MAX as f32) as u16
}

fn get_base_effects(
    GamepadRumbleIntensity {
        weak_motor,
        strong_motor,
    }: GamepadRumbleIntensity,
    duration: Duration,
) -> Vec<ff::BaseEffect> {
    let mut effects = Vec::new();
    if strong_motor > 0. {
        effects.push(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: to_gilrs_magnitude(strong_motor),
            },
            scheduling: Replay {
                play_for: duration.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    }
    if weak_motor > 0. {
        effects.push(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: to_gilrs_magnitude(weak_motor),
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
    current_time: Duration,
) -> Result<(), RumbleError> {
    let gamepad = rumble.gamepad();

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
            let mut effect_builder = ff::EffectBuilder::new();

            for effect in get_base_effects(intensity, duration) {
                effect_builder.add_effect(effect);
                effect_builder.repeat(Repeat::For(duration.into()));
            }

            let effect = effect_builder.gamepads(&[gamepad_id]).finish(gilrs)?;
            effect.play()?;

            let gamepad_rumbles = manager.rumbles.entry(gamepad_id).or_default();
            let deadline = current_time + duration;
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
    let current_time = time.raw_elapsed();
    // Remove outdated rumble effects.
    for rumbles in manager.rumbles.values_mut() {
        // `ff::Effect` uses RAII, dropping = deactivating
        rumbles.retain(|RunningRumble { deadline, .. }| *deadline >= current_time);
    }
    manager
        .rumbles
        .retain(|_gamepad, rumbles| !rumbles.is_empty());

    // Add new effects.
    for rumble in requests.iter().cloned() {
        let gamepad = rumble.gamepad();
        match handle_rumble_request(&mut manager, &mut gilrs, rumble, current_time) {
            Ok(()) => {}
            Err(RumbleError::GilrsError(err)) => {
                if let ff::Error::FfNotSupported(_) = err {
                    debug!("Tried to rumble {gamepad:?}, but it doesn't support force feedback");
                } else {
                    warn!(
                    "Tried to handle rumble request for {gamepad:?} but an error occurred: {err}"
                    );
                }
            }
            Err(RumbleError::GamepadNotFound) => {
                warn!("Tried to handle rumble request {gamepad:?} but it doesn't exist!");
            }
        };
    }
}
