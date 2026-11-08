//! `input.*` verbs for the Bevy Remote Protocol.

use bevy_dev_tools::synthetic_input::{self, ClickTarget, KeyAction};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    entity::Entity,
    message::Messages,
    resource::Resource,
    system::{In, Res, ResMut},
    world::World,
};
use bevy_input::{
    keyboard::{Key, KeyCode},
    mouse::MouseButton,
    ButtonState,
};
use bevy_math::Vec2;
use bevy_window::WindowEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{builtin_methods::parse_some, error_codes, BrpError, BrpResponder, BrpResult};

/// The method path for an `input.click` request.
pub const BRP_CLICK_METHOD: &str = "input.click";

/// `input.click`: Clicks on a UI node, or at a point in a window.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with a [`BrpClickResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpClickParams {
    /// The UI node to click at the center of. A node hidden by a scrolled ancestor is scrolled
    /// into view first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to click in, with `position`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to click in `window`, in physical pixels from its top left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// The mouse button to click with. Defaults to [`MouseButton::Left`].
    #[serde(default = "left")]
    pub button: MouseButton,

    /// How many frames the button is held down for. Defaults to 1.
    #[serde(default = "one")]
    pub hold: u32,
}

fn left() -> MouseButton {
    MouseButton::Left
}

fn one() -> u32 {
    1
}

/// The response to an `input.click` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpClickResponse {
    /// The frame on which the app receives the button's release, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// Handles an `input.click` request coming from a client, answering once the app has run the frame
/// that receives the button's release.
pub fn process_remote_click_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let response = queue_click(params, world).map(|response| (response.frame, response));
    respond_once_received(world, responder, response);
}

fn queue_click(params: Option<Value>, world: &mut World) -> BrpResult<BrpClickResponse> {
    can_receive_input(world)?;
    let BrpClickParams {
        entity,
        window,
        position,
        button,
        hold,
    } = parse_some(params)?;
    let target = match (entity, window, position) {
        (Some(entity), None, None) => ClickTarget::Node(entity),
        (None, Some(window), Some(position)) => ClickTarget::Point { window, position },
        _ => {
            return Err(invalid_params(
                "a click takes an `entity`, or a `window` and a `position`".into(),
            ))
        }
    };
    let frame = synthetic_input::click(world, target, button, hold)
        .map_err(|err| invalid_params(err.to_string()))?;
    Ok(BrpClickResponse { frame })
}

/// The method path for an `input.key` request.
pub const BRP_KEY_METHOD: &str = "input.key";

/// The method path for an `input.type` request.
pub const BRP_TYPE_METHOD: &str = "input.type";

/// `input.key`: Presses a key, and releases it `hold` frames later, or with `state`, only
/// presses or only releases it.
///
/// The server responds with a [`BrpKeyResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpKeyParams {
    /// The physical key.
    pub key_code: KeyCode,

    /// What the key means in the keyboard layout. When omitted, this and `text` come from a US
    /// layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_key: Option<Key>,

    /// The text the key types, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Only press the key, or only release it. When omitted, the key is pressed and released.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ButtonState>,

    /// How many frames the key is held down for, when it is pressed and released. Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<u32>,

    /// The window to send the key to. Defaults to the primary window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,
}

/// The response to an `input.key` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpKeyResponse {
    /// The frame on which the app receives the last key event, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// `input.type`: Types text as a US keyboard would, holding Shift where a character needs it.
///
/// The server responds with a [`BrpTypeResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpTypeParams {
    /// The text to type.
    pub text: String,

    /// The window to type in. Defaults to the primary window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,
}

/// The response to an `input.type` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpTypeResponse {
    /// The frame on which the app receives the last key event, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// Handles an `input.key` request coming from a client, answering once the app has run the frame
/// that receives the last key event.
pub fn process_remote_key_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let response = queue_key(params, world).map(|response| (response.frame, response));
    respond_once_received(world, responder, response);
}

fn queue_key(params: Option<Value>, world: &mut World) -> BrpResult<BrpKeyResponse> {
    can_receive_input(world)?;
    let BrpKeyParams {
        key_code,
        logical_key,
        text,
        state,
        hold,
        window,
    } = parse_some(params)?;
    let action = match (state, hold) {
        (None, hold) => KeyAction::Tap {
            hold: hold.unwrap_or(1),
        },
        (Some(ButtonState::Pressed), None) => KeyAction::Press,
        (Some(ButtonState::Released), None) => KeyAction::Release,
        (Some(_), Some(_)) => {
            return Err(invalid_params(
                "a key takes a `state` or a `hold`, not both".into(),
            ))
        }
    };
    let frame = synthetic_input::key(
        world,
        window,
        key_code,
        logical_key,
        text.as_deref(),
        action,
    )
    .map_err(|err| invalid_params(err.to_string()))?;
    Ok(BrpKeyResponse { frame })
}

/// Handles an `input.type` request coming from a client, answering once the app has run the frame
/// that receives the last key event.
pub fn process_remote_type_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let response = queue_type(params, world).map(|response| (response.frame, response));
    respond_once_received(world, responder, response);
}

fn queue_type(params: Option<Value>, world: &mut World) -> BrpResult<BrpTypeResponse> {
    can_receive_input(world)?;
    let BrpTypeParams { text, window } = parse_some(params)?;
    let frame = synthetic_input::type_text(world, window, &text)
        .map_err(|err| invalid_params(err.to_string()))?;
    Ok(BrpTypeResponse { frame })
}

#[derive(Resource, Default)]
pub(crate) struct PendingInputResponses(Vec<PendingInputResponse>);

struct PendingInputResponse {
    /// The frame that receives the input's last event.
    frame: u32,
    responder: BrpResponder,
    response: Value,
}

fn can_receive_input(world: &World) -> BrpResult<()> {
    if world.contains_resource::<FrameCount>() && world.contains_resource::<Messages<WindowEvent>>()
    {
        Ok(())
    } else {
        Err(BrpError {
            code: error_codes::INTERNAL_ERROR,
            message:
                "the app can't receive input: it needs `FrameCount` and `WindowEvent` messages"
                    .into(),
            data: None,
        })
    }
}

/// Answers with `response` once the app has run the frame it names, or with its error straight
/// away.
fn respond_once_received<R: Serialize>(
    world: &mut World,
    responder: BrpResponder,
    response: BrpResult<(u32, R)>,
) {
    let response = response.and_then(|(frame, response)| {
        Ok((
            frame,
            serde_json::to_value(response).map_err(BrpError::internal)?,
        ))
    });
    match response {
        Ok((frame, response)) => world
            .get_resource_or_init::<PendingInputResponses>()
            .0
            .push(PendingInputResponse {
                frame,
                responder,
                response,
            }),
        Err(err) => responder.respond(Err(err)),
    }
}

/// Answers the input requests whose last event was received on a frame the app has finished.
///
/// Run it after [`Last`](bevy_app::Last), where [`FrameCount`] advances past the frame that just
/// ran.
pub(crate) fn answer_received_input(
    frame: Res<FrameCount>,
    mut pending: ResMut<PendingInputResponses>,
) {
    let (received, waiting): (Vec<_>, Vec<_>) = core::mem::take(&mut pending.0)
        .into_iter()
        .filter(|pending| !pending.responder.is_closed())
        .partition(|pending| pending.frame < frame.0);
    pending.0 = waiting;
    for pending in received {
        pending.responder.respond(Ok(pending.response));
    }
}

fn invalid_params(message: String) -> BrpError {
    BrpError {
        code: error_codes::INVALID_PARAMS,
        message,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_diagnostic::FrameCount;
    use bevy_ui::{ComputedUiTargetCamera, UiGlobalTransform};
    use bevy_window::{PrimaryWindow, Window};
    use serde_json::json;

    fn world() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(FrameCount(10));
        world.init_resource::<Messages<WindowEvent>>();
        let window = world.spawn((Window::default(), PrimaryWindow)).id();
        (world, window)
    }

    fn click(world: &mut World, params: Value) -> BrpResult {
        queue_click(Some(params), world).map(|response| serde_json::to_value(response).unwrap())
    }

    #[test]
    fn a_click_answers_with_the_frame_of_its_release() {
        let (mut world, window) = world();
        let node = world
            .spawn((
                UiGlobalTransform::from_translation(Vec2::new(400.0, 300.0)),
                ComputedUiTargetCamera::default(),
            ))
            .id();

        let found = click(&mut world, json!({ "entity": node, "hold": 3 }));
        assert_eq!(found.unwrap(), json!({ "frame": 14 }));

        let params = json!({ "window": window, "position": [5.0, 6.0], "button": "Right" });
        assert_eq!(click(&mut world, params).unwrap(), json!({ "frame": 17 }));
    }

    #[test]
    fn the_remote_plugin_installs_the_input_queue() {
        let mut app = bevy_app::App::new();
        app.add_plugins(crate::RemotePlugin::default());

        assert!(app
            .world()
            .contains_resource::<synthetic_input::InputQueue>());
    }

    #[test]
    fn a_click_needs_one_target() {
        let (mut world, window) = world();
        let other = world.spawn_empty().id();

        for params in [
            json!({}),
            json!({ "window": window }),
            json!({ "entity": other, "window": window, "position": [1.0, 1.0] }),
            json!({ "entity": other }),
            json!({ "window": window, "position": [1.0, 1.0], "hold": 0 }),
            json!({ "window": window, "position": [1.0, 1.0], "button": "Thumb" }),
        ] {
            let err = click(&mut world, params.clone()).unwrap_err();
            assert_eq!(err.code, error_codes::INVALID_PARAMS, "{params}");
        }
    }

    #[test]
    fn keys_and_text_answer_with_the_frame_of_their_last_event() {
        let (mut world, _) = world();
        let key = |world: &mut World, params| {
            queue_key(Some(params), world).map(|response| serde_json::to_value(response).unwrap())
        };

        let tapped = key(&mut world, json!({ "key_code": "KeyA" }));
        assert_eq!(tapped.unwrap(), json!({ "frame": 11 }));
        let pressed = key(
            &mut world,
            json!({ "key_code": "ShiftLeft", "state": "Pressed" }),
        );
        assert_eq!(pressed.unwrap(), json!({ "frame": 12 }));
        let typed = queue_type(Some(json!({ "text": "a" })), &mut world)
            .map(|response| serde_json::to_value(response).unwrap());
        assert_eq!(typed.unwrap(), json!({ "frame": 14 }));
        let plus = json!({
            "key_code": "NumpadAdd",
            "logical_key": { "Character": "+" },
            "text": "+",
            "hold": 2,
        });
        assert_eq!(key(&mut world, plus).unwrap(), json!({ "frame": 17 }));
    }

    #[test]
    fn a_key_needs_a_meaning_and_one_of_state_or_hold() {
        let (mut world, _) = world();

        for params in [
            json!({}),
            json!({ "key_code": "NotAKey" }),
            json!({ "key_code": "NumpadAdd" }),
            json!({ "key_code": "KeyA", "state": "Pressed", "hold": 2 }),
            json!({ "key_code": "KeyA", "hold": 0 }),
        ] {
            let err = queue_key(Some(params.clone()), &mut world).unwrap_err();
            assert_eq!(err.code, error_codes::INVALID_PARAMS, "{params}");
        }
        let err = queue_type(Some(json!({ "text": "" })), &mut world).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
    }
}
