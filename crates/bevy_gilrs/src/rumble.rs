//! Handle user specified rumble request events.
use bevy_ecs::{
    prelude::{EventReader, Res},
    system::NonSendMut,
};
use bevy_input::gamepad::{GamepadRumbleRequest, RumbleIntensity};
use bevy_log::{debug, warn};
use bevy_time::Time;
use gilrs::{ff, GamepadId, Gilrs};

use crate::converter::convert_gamepad_id;

fn get_effect_type(intensity: RumbleIntensity) -> ff::BaseEffectType {
    use RumbleIntensity::*;
    match intensity {
        Strong => ff::BaseEffectType::Strong { magnitude: 63_000 },
        Medium => ff::BaseEffectType::Strong { magnitude: 40_000 },
        Weak => ff::BaseEffectType::Weak { magnitude: 40_000 },
    }
}

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
    rumbles: Vec<(GamepadId, RunningRumble)>,
}

fn add_rumble(
    manager: &mut RumblesManager,
    gilrs: &mut Gilrs,
    rumble: GamepadRumbleRequest,
    current_time: f32,
) -> Result<(), RumbleError> {
    let (pad_id, _) = gilrs
        .gamepads()
        .find(|(pad_id, _)| convert_gamepad_id(*pad_id) == rumble.gamepad)
        .ok_or(RumbleError::GamepadNotFound)?;
    let deadline = current_time + rumble.duration_seconds;

    let kind = get_effect_type(rumble.intensity);
    let effect = ff::BaseEffect {
        kind,
        ..Default::default()
    };
    let mut effect_builder = ff::EffectBuilder::new();
    let effect_builder = effect_builder.add_effect(effect);

    let effect = effect_builder.gamepads(&[pad_id]).finish(gilrs)?;
    effect.play()?;
    manager
        .rumbles
        .push((pad_id, RunningRumble { deadline, effect }));
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
    // `ff::Effect` uses RAII, dropping = deactivating
    manager
        .rumbles
        .retain(|(_, RunningRumble { deadline, .. })| *deadline >= current_time);

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
