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

fn is_stop_request(request: &GamepadRumbleRequest) -> bool {
    !request.additive && request.intensity == GamepadRumbleIntensity::ZERO
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

fn add_rumble(
    manager: &mut RumblesManager,
    gilrs: &mut Gilrs,
    rumble: GamepadRumbleRequest,
    current_time: f32,
) -> Result<(), RumbleError> {
    let (gamepad_id, _) = gilrs
        .gamepads()
        .find(|(pad_id, _)| convert_gamepad_id(*pad_id) == rumble.gamepad)
        .ok_or(RumbleError::GamepadNotFound)?;

    if !rumble.additive {
        // `ff::Effect` uses RAII, dropping = deactivating
        manager.rumbles.remove(&gamepad_id);
    }

    if !is_stop_request(&rumble) {
        let deadline = current_time + rumble.duration_seconds;

        let mut effect_builder = ff::EffectBuilder::new();

        for effect in get_base_effects(rumble.intensity) {
            effect_builder.add_effect(effect);
        }

        let effect = effect_builder.gamepads(&[gamepad_id]).finish(gilrs)?;
        effect.play()?;

        let gamepad_rumbles = manager.rumbles.entry(gamepad_id).or_default();
        gamepad_rumbles.push(RunningRumble { deadline, effect });
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
        let pad = rumble.gamepad;
        match add_rumble(&mut manager, &mut gilrs, rumble, current_time) {
            Ok(()) => {}
            Err(RumbleError::GilrsError(err)) => {
                debug!("Tried to rumble {pad:?} but an error occurred: {err}");
            }
            Err(RumbleError::GamepadNotFound) => {
                warn!("Tried to rumble {pad:?} but it doesn't exist!");
            }
        };
    }
}
