//! Handle user specified Rumble request events.
use crate::converter::convert_gamepad_id;
use bevy_app::EventReader;
use bevy_core::Time;
use bevy_ecs::{prelude::Res, system::NonSendMut};
use bevy_input::gamepad::Gamepad;
use bevy_log as log;
use bevy_utils::HashMap;
use gilrs::{ff, GamepadId, Gilrs};

pub enum RumbleIntensity {
    Strong,
    Medium,
    Weak,
}
impl RumbleIntensity {
    fn effect_type(&self) -> ff::BaseEffectType {
        use RumbleIntensity::*;
        match self {
            Strong => ff::BaseEffectType::Strong { magnitude: 63_000 },
            Medium => ff::BaseEffectType::Strong { magnitude: 40_000 },
            Weak => ff::BaseEffectType::Weak { magnitude: 40_000 },
        }
    }
}

/// Request `pad` rumble in `gilrs_effect` pattern for `duration_seconds`
///
/// # Notes
///
/// * Does nothing if `pad` does not support rumble
/// * If a new `RumbleRequest` is sent while another one is still executing, it
///   replaces the old one.
///
/// # Example
///
/// ```
/// # use bevy_gilrs::{RumbleRequest, RumbleIntensity};
/// # use bevy_input::gamepad::Gamepad;
/// # use bevy_app::EventWriter;
/// fn rumble_pad_system(mut rumble_requests: EventWriter<RumbleRequest>) {
///     let request = RumbleRequest::with_intensity(
///         RumbleIntensity::Strong,
///         10.0,
///         Gamepad(0),
///     );
///     rumble_requests.send(request);
/// }
/// ```
#[derive(Clone)]
pub struct RumbleRequest {
    /// The duration in seconds of the rumble
    pub duration_seconds: f32,
    /// The gilrs descriptor, use [`RumbleRequest::with_intensity`] if you want
    /// a simpler API.
    pub gilrs_effect: ff::EffectBuilder,
    /// The gamepad to rumble
    pub pad: Gamepad,
}
impl RumbleRequest {
    /// Causes `pad` to rumble for `duration_seconds` at given `intensity`.
    pub fn with_intensity(intensity: RumbleIntensity, duration_seconds: f32, pad: Gamepad) -> Self {
        let kind = intensity.effect_type();
        let effect = ff::BaseEffect {
            kind,
            ..Default::default()
        };
        let mut gilrs_effect = ff::EffectBuilder::new();
        gilrs_effect.add_effect(effect);
        RumbleRequest {
            duration_seconds,
            gilrs_effect,
            pad,
        }
    }
    /// Stops provided `pad` rumbling.
    pub fn stop(pad: Gamepad) -> Self {
        RumbleRequest {
            duration_seconds: 0.0,
            gilrs_effect: ff::EffectBuilder::new(),
            pad,
        }
    }
}

struct RunningRumble {
    deadline: f32,
    // We use `effect.drop()` to interact with this, but rustc can't know
    // gilrs uses Drop as an API feature.
    #[allow(dead_code)]
    effect: ff::Effect,
}

#[derive(Default)]
pub(crate) struct RumblesManager {
    rumbles: HashMap<GamepadId, RunningRumble>,
}

pub(crate) fn gilrs_rumble_system(
    time: Res<Time>,
    mut gilrs: NonSendMut<Gilrs>,
    mut requests: EventReader<RumbleRequest>,
    mut manager: NonSendMut<RumblesManager>,
) {
    let current_time = time.seconds_since_startup() as f32;
    // Remove outdated rumble effects.
    if !manager.rumbles.is_empty() {
        let mut to_remove = Vec::new();
        for (id, RunningRumble { deadline, .. }) in manager.rumbles.iter() {
            if *deadline < current_time {
                to_remove.push(*id);
            }
        }
        for id in &to_remove {
            // `ff::Effect` uses RAII, dropping = deactivating
            manager.rumbles.remove(id);
        }
    }
    // Add new effects.
    for mut rumble in requests.iter().cloned() {
        let gilrs_pad = gilrs
            .gamepads()
            .find(|(pad_id, _)| convert_gamepad_id(*pad_id) == rumble.pad);
        if let Some((pad_id, _)) = gilrs_pad {
            let current_time = time.seconds_since_startup() as f32;
            let deadline = current_time + rumble.duration_seconds;
            let effect = rumble.gilrs_effect.gamepads(&[pad_id]).finish(&mut gilrs);
            match effect {
                Ok(effect) => {
                    if let Err(err) = effect.play() {
                        log::error!(
                            "Tried to rumble {:?} but an error occurred: {err}",
                            rumble.pad
                        );
                        continue;
                    };
                    manager
                        .rumbles
                        .insert(pad_id, RunningRumble { deadline, effect });
                }
                Err(err) => {
                    log::debug!(
                        "Tried to rumble {:?} but an error occurred: {err}",
                        rumble.pad
                    );
                }
            }
        } else {
            log::warn!(
                "Tried to trigger rumble on gamepad {:?}, but that gamepad doesn't exist",
                rumble.pad,
            );
        }
    }
}
