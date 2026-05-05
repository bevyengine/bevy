//! Handle user specified rumble request events.
use crate::{Gilrs, GilrsGamepads};
use bevy_ecs::prelude::{MessageReader, Res, ResMut, Resource};
use bevy_input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy_platform::cell::SyncCell;
use bevy_platform::collections::HashMap;
use bevy_time::{Real, Time};
use core::time::Duration;
use gilrs::{
    ff::{self, BaseEffect, BaseEffectType, Repeat, Replay},
    GamepadId,
};
use thiserror::Error;
use tracing::{debug, warn};

/// A rumble effect that is currently in effect.
struct RunningRumble {
    /// Duration from app startup when this effect will be finished
    deadline: Duration,
    /// A ref-counted handle to the specific force-feedback effect
    ///
    /// Dropping it will cause the effect to stop
    #[expect(
        dead_code,
        reason = "We don't need to read this field, as its purpose is to keep the rumble effect going until the field is dropped."
    )]
    effect: SyncCell<ff::Effect>,
}

#[derive(Error, Debug)]
enum RumbleError {
    #[error("gamepad not found")]
    GamepadNotFound,
    #[error("gilrs error while rumbling gamepad: {0}")]
    GilrsError(#[from] ff::Error),
}

/// Contains the gilrs rumble effects that are currently running for each gamepad
#[derive(Default, Resource)]
pub(crate) struct RunningRumbleEffects {
    /// If multiple rumbles are running at the same time, their resulting rumble
    /// will be the saturated sum of their strengths up until [`u16::MAX`]
    rumbles: HashMap<GamepadId, Vec<RunningRumble>>,
}

/// gilrs uses magnitudes from 0 to [`u16::MAX`], while ours go from `0.0` to `1.0` ([`f32`])
fn to_gilrs_magnitude(ratio: f32) -> u16 {
    (ratio * u16::MAX as f32) as u16
}

fn get_base_effects(
    GamepadRumbleIntensity {
        weak_motor,
        strong_motor,
    }: GamepadRumbleIntensity,
    duration: Duration,
) -> Vec<BaseEffect> {
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
    running_rumbles: &mut RunningRumbleEffects,
    gilrs: &mut gilrs::Gilrs,
    gamepads: &GilrsGamepads,
    rumble: GamepadRumbleRequest,
    current_time: Duration,
) -> Result<(), RumbleError> {
    let gamepad = rumble.gamepad();

    let (gamepad_id, _) = gilrs
        .gamepads()
        .find(|(pad_id, _)| *pad_id == gamepads.get_gamepad_id(gamepad).unwrap())
        .ok_or(RumbleError::GamepadNotFound)?;

    match rumble {
        GamepadRumbleRequest::Stop { .. } => {
            // `ff::Effect` uses RAII, dropping = deactivating
            running_rumbles.rumbles.remove(&gamepad_id);
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

            let gamepad_rumbles = running_rumbles.rumbles.entry(gamepad_id).or_default();
            let deadline = current_time + duration;
            gamepad_rumbles.push(RunningRumble {
                deadline,
                effect: SyncCell::new(effect),
            });
        }
    }

    Ok(())
}
pub(crate) fn play_gilrs_rumble(
    time: Res<Time<Real>>,
    mut gilrs: ResMut<Gilrs>,
    gamepads: Res<GilrsGamepads>,
    mut requests: MessageReader<GamepadRumbleRequest>,
    mut running_rumbles: ResMut<RunningRumbleEffects>,
) {
    gilrs.with(|gilrs| {
        let current_time = time.elapsed();
        // Remove outdated rumble effects.
        for rumbles in running_rumbles.rumbles.values_mut() {
            // `ff::Effect` uses RAII, dropping = deactivating
            rumbles.retain(|RunningRumble { deadline, .. }| *deadline >= current_time);
        }
        running_rumbles
            .rumbles
            .retain(|_gamepad, rumbles| !rumbles.is_empty());

        // Add new effects.
        for rumble in requests.read().cloned() {
            let gamepad = rumble.gamepad();
            match handle_rumble_request(&mut running_rumbles, gilrs, &gamepads, rumble, current_time) {
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
    });
}

#[cfg(test)]
mod tests {
    use super::to_gilrs_magnitude;

    #[test]
    fn magnitude_conversion() {
        assert_eq!(to_gilrs_magnitude(1.0), u16::MAX);
        assert_eq!(to_gilrs_magnitude(0.0), 0);

        // bevy magnitudes of 2.0 don't really make sense, but just make sure
        // they convert to something sensible in gilrs anyway.
        assert_eq!(to_gilrs_magnitude(2.0), u16::MAX);

        // negative bevy magnitudes don't really make sense, but just make sure
        // they convert to something sensible in gilrs anyway.
        assert_eq!(to_gilrs_magnitude(-1.0), 0);
        assert_eq!(to_gilrs_magnitude(-0.1), 0);
    }
}
