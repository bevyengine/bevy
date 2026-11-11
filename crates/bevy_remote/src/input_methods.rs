//! `input.*` verbs for the Bevy Remote Protocol.

use bevy_dev_tools::synthetic_input::{self, ClickTarget, InputQueue, KeyAction, QueuedInputId};
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
/// The server responds with `null` once the app has run the frame that receives the button's
/// release.
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

/// Handles an `input.click` request coming from a client, answering once the app has run the frame
/// that receives the button's release.
pub fn process_remote_click_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let queued = queue_click(params, world);
    respond_once_received(world, responder, queued);
}

fn queue_click(params: Option<Value>, world: &mut World) -> BrpResult<QueuedInputId> {
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
    synthetic_input::click(world, target, button, hold)
        .map_err(|err| invalid_params(err.to_string()))
}

/// The method path for an `input.key` request.
pub const BRP_KEY_METHOD: &str = "input.key";

/// The method path for an `input.type` request.
pub const BRP_TYPE_METHOD: &str = "input.type";

/// `input.key`: Presses a key, and releases it `hold` frames later, or with `state`, only
/// presses or only releases it.
///
/// The server responds with `null` once the app has run the frame that receives the last key
/// event.
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

/// `input.type`: Types text as a US keyboard would, holding Shift where a character needs it.
///
/// The server responds with `null` once the app has run the frame that receives the last key
/// event.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpTypeParams {
    /// The text to type.
    pub text: String,

    /// The window to type in. Defaults to the primary window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,
}

/// Handles an `input.key` request coming from a client, answering once the app has run the frame
/// that receives the last key event.
pub fn process_remote_key_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let queued = queue_key(params, world);
    respond_once_received(world, responder, queued);
}

fn queue_key(params: Option<Value>, world: &mut World) -> BrpResult<QueuedInputId> {
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
    synthetic_input::key(
        world,
        window,
        key_code,
        logical_key,
        text.as_deref(),
        action,
    )
    .map_err(|err| invalid_params(err.to_string()))
}

/// Handles an `input.type` request coming from a client, answering once the app has run the frame
/// that receives the last key event.
pub fn process_remote_type_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let queued = queue_type(params, world);
    respond_once_received(world, responder, queued);
}

fn queue_type(params: Option<Value>, world: &mut World) -> BrpResult<QueuedInputId> {
    can_receive_input(world)?;
    let BrpTypeParams { text, window } = parse_some(params)?;
    synthetic_input::type_text(world, window, &text).map_err(|err| invalid_params(err.to_string()))
}

#[derive(Resource, Default)]
pub(crate) struct PendingInputResponses(Vec<PendingInputResponse>);

struct PendingInputResponse {
    /// The input's last event.
    input: QueuedInputId,
    responder: BrpResponder,
}

fn can_receive_input(world: &World) -> BrpResult<()> {
    if world.contains_resource::<Messages<WindowEvent>>() {
        Ok(())
    } else {
        Err(BrpError::internal(
            "the app can't receive input: it has no `WindowEvent` messages",
        ))
    }
}

/// Answers once the app has run the frame that receives the `queued` input, or with its error
/// straight away.
fn respond_once_received(
    world: &mut World,
    responder: BrpResponder,
    queued: BrpResult<QueuedInputId>,
) {
    match queued {
        Ok(input) => world
            .get_resource_or_init::<PendingInputResponses>()
            .0
            .push(PendingInputResponse { input, responder }),
        Err(err) => responder.respond(Err(err)),
    }
}

/// Answers the input requests whose last event the app has read.
///
/// Run it once a frame, right before [`write_due_input`](synthetic_input::write_due_input): an
/// event written on an earlier run has been read by the frame that ran since. Requests whose client
/// has gone are dropped.
pub(crate) fn answer_received_input(
    queue: Res<InputQueue>,
    mut pending: ResMut<PendingInputResponses>,
) {
    for request in pending.0.extract_if(.., |request| {
        request.responder.is_closed() || queue.is_written(request.input)
    }) {
        request.responder.respond(Ok(Value::Null));
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
    use async_channel::Receiver;
    use bevy_diagnostic::FrameCount;
    use bevy_ui::{ComputedUiTargetCamera, UiGlobalTransform};
    use bevy_window::{PrimaryWindow, Window};
    use serde_json::json;

    fn world() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(FrameCount(10));
        world.init_resource::<Messages<WindowEvent>>();
        world.init_resource::<InputQueue>();
        world.init_resource::<PendingInputResponses>();
        let window = world.spawn((Window::default(), PrimaryWindow)).id();
        (world, window)
    }

    /// Sends a request to an `input.*` handler.
    fn send(
        world: &mut World,
        handler: fn(In<(Option<Value>, BrpResponder)>, &mut World),
        params: Value,
    ) -> Receiver<BrpResult> {
        let (sender, receiver) = async_channel::bounded(1);
        handler(In((Some(params), BrpResponder::new(sender))), world);
        receiver
    }

    /// Runs frames, as [`RemotePlugin`](crate::RemotePlugin) does after `Last`, until `request` is
    /// answered. Returns the answer, and the last frame the app had run when it came.
    fn answered(world: &mut World, request: Receiver<BrpResult>) -> (BrpResult, u32) {
        for _ in 0..100 {
            // After `Last`, `FrameCount` names the frame about to run.
            let ran = world.resource::<FrameCount>().0 - 1;
            world.run_system_cached(answer_received_input).unwrap();
            world
                .run_system_cached(synthetic_input::write_due_input)
                .unwrap();
            world.resource_mut::<FrameCount>().0 += 1;
            if let Ok(answer) = request.try_recv() {
                return (answer, ran);
            }
        }
        panic!("no answer after 100 frames");
    }

    fn click(world: &mut World, params: Value) -> BrpResult {
        let request = send(world, process_remote_click_request, params);
        answered(world, request).0
    }

    #[test]
    fn a_click_answers_after_the_frame_of_its_release() {
        let (mut world, window) = world();
        let node = world
            .spawn((
                UiGlobalTransform::from_translation(Vec2::new(400.0, 300.0)),
                ComputedUiTargetCamera::default(),
            ))
            .id();

        let found = send(
            &mut world,
            process_remote_click_request,
            json!({ "entity": node, "hold": 3 }),
        );
        let params = json!({ "window": window, "position": [5.0, 6.0], "button": "Right" });
        let point = send(&mut world, process_remote_click_request, params);

        let (answer, ran) = answered(&mut world, found);
        assert_eq!((answer.unwrap(), ran), (Value::Null, 14));
        let (answer, ran) = answered(&mut world, point);
        assert_eq!((answer.unwrap(), ran), (Value::Null, 17));
    }

    #[test]
    fn the_remote_plugin_installs_the_input_queue() {
        let mut app = bevy_app::App::new();
        app.add_plugins(crate::RemotePlugin::default());

        assert!(app.world().contains_resource::<InputQueue>());
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
    fn keys_and_text_answer_after_the_frame_of_their_last_event() {
        let (mut world, _) = world();
        let key = |world: &mut World, params| send(world, process_remote_key_request, params);

        let tapped = key(&mut world, json!({ "key_code": "KeyA" }));
        let pressed = key(
            &mut world,
            json!({ "key_code": "ShiftLeft", "state": "Pressed" }),
        );
        let typed = send(
            &mut world,
            process_remote_type_request,
            json!({ "text": "a" }),
        );
        let plus = key(
            &mut world,
            json!({
                "key_code": "NumpadAdd",
                "logical_key": { "Character": "+" },
                "text": "+",
                "hold": 2,
            }),
        );

        for (request, frame) in [(tapped, 11), (pressed, 12), (typed, 14), (plus, 17)] {
            let (answer, ran) = answered(&mut world, request);
            assert_eq!((answer.unwrap(), ran), (Value::Null, frame));
        }
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
