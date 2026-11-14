//! `input.*` verbs for the Bevy Remote Protocol.

use bevy_curve::EaseFunction;
use bevy_dev_tools::synthetic_input::{self, ButtonAction, ClickTarget, InputQueue, QueuedInputId};
use bevy_ecs::{
    entity::Entity,
    message::Messages,
    resource::Resource,
    system::{In, Res, ResMut},
    world::World,
};
use bevy_input::{
    keyboard::{Key, KeyCode},
    mouse::{MouseButton, MouseScrollUnit},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_window::WindowEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{builtin_methods::parse_some, error_codes, BrpError, BrpResponder, BrpResult};

/// The method path for an `input.click` request.
pub const BRP_CLICK_METHOD: &str = "input.click";

/// `input.click`: Clicks on a UI node, or at a point in a window, or with `state`, only presses or
/// only releases the button there.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with `null` once the app has run the frame that receives the button's last
/// event.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpClickParams {
    /// The UI node to click at the center of. A node hidden by a scrolled ancestor is scrolled
    /// into view first. When omitted, the click lands at `position` in `window`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to click in, with `position`. Required when `entity` is omitted, and not
    /// allowed with it: a node is clicked in the window it's drawn in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to click in `window`, in physical pixels from its top left. Required when
    /// `entity` is omitted, and not allowed with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// The mouse button to click with. Defaults to [`MouseButton::Left`].
    #[serde(default = "left")]
    pub button: MouseButton,

    /// Only press the button, or only release it. When omitted, the button is pressed and
    /// released.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ButtonState>,

    /// How many frames the button is held down for, when it is pressed and released. When
    /// omitted, it's held for 1 frame. Not allowed with `state`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<u32>,
}

fn left() -> MouseButton {
    MouseButton::Left
}

/// Handles an `input.click` request coming from a client, answering once the app has run the frame
/// that receives the button's last event.
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
        state,
        hold,
    } = parse_some(params)?;
    let target = pointer_target(entity, window, position)?;
    let action = button_action(state, hold)
        .ok_or_else(|| invalid_params("a click takes a `state` or a `hold`, not both".into()))?;
    synthetic_input::click(world, target, button, action)
        .map_err(|err| invalid_params(err.to_string()))
}

/// The method path for an `input.move_cursor` request.
pub const BRP_MOVE_CURSOR_METHOD: &str = "input.move_cursor";

/// The method path for an `input.scroll` request.
pub const BRP_SCROLL_METHOD: &str = "input.scroll";

/// `input.move_cursor`: Moves the mouse cursor to a UI node, or to a point in a window, without
/// pressing anything.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with `null` once the app has run the frame that receives the cursor's last
/// move.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpMoveCursorParams {
    /// The UI node to move the mouse cursor to the center of. A node hidden by a scrolled ancestor
    /// is scrolled into view first. When omitted, the cursor moves to `position` in `window`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to move the mouse cursor in, with `position`. Required when `entity` is
    /// omitted, and not allowed with it: the cursor moves in the window the node is drawn in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to move the mouse cursor to in `window`, in physical pixels from its top left.
    /// Required when `entity` is omitted, and not allowed with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// How many frames the move is spread over, from 1 to 255. The app receives one cursor
    /// position per frame, each a step further along the straight line from where the cursor was
    /// to the target, and the last one is the target itself. When omitted, this is 1: the cursor
    /// jumps straight to the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u8>,

    /// How far along the line the cursor is on each frame of a move spread over several
    /// `frames`. With [`EaseFunction::Linear`], every step is the same length. Other functions
    /// make the steps longer or shorter at the start or end of the move, so
    /// [`EaseFunction::QuadraticInOut`] starts slowly, speeds up, and slows down again before the
    /// target. When omitted, this is [`EaseFunction::Linear`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease: Option<EaseFunction>,
}

/// `input.scroll`: Turns the mouse wheel over a UI node, or over a point in a window.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with `null` once the app has run the frame that receives the wheel's turn.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpScrollParams {
    /// The UI node to scroll over the center of. A node hidden by a scrolled ancestor is scrolled
    /// into view first. When omitted, the wheel turns over `position` in `window`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to scroll in, with `position`. Required when `entity` is omitted, and not
    /// allowed with it: the wheel turns in the window the node is drawn in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to scroll over in `window`, in physical pixels from its top left. Required when
    /// `entity` is omitted, and not allowed with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// How far the wheel turns, horizontally and vertically, in `unit`s. A positive `y` scrolls
    /// up, while a positive `x` scrolls right.
    pub delta: Vec2,

    /// The scroll units used by `delta`. Defaults to [`MouseScrollUnit::Line`].
    #[serde(default = "line")]
    pub unit: MouseScrollUnit,
}

fn line() -> MouseScrollUnit {
    MouseScrollUnit::Line
}

/// Handles an `input.move_cursor` request coming from a client, answering once the app has run the
/// frame that receives the cursor's last move.
pub fn process_remote_move_cursor_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let queued = queue_move_cursor(params, world);
    respond_once_received(world, responder, queued);
}

fn queue_move_cursor(params: Option<Value>, world: &mut World) -> BrpResult<QueuedInputId> {
    can_receive_input(world)?;
    let BrpMoveCursorParams {
        entity,
        window,
        position,
        frames,
        ease,
    } = parse_some(params)?;
    let target = pointer_target(entity, window, position)?;
    let frames = frames.unwrap_or(1);
    let ease = ease.unwrap_or(EaseFunction::Linear);
    synthetic_input::move_cursor(world, target, frames, ease)
        .map_err(|err| invalid_params(err.to_string()))
}

/// Handles an `input.scroll` request coming from a client, answering once the app has run the
/// frame that receives the wheel's turn.
pub fn process_remote_scroll_request(
    In((params, responder)): In<(Option<Value>, BrpResponder)>,
    world: &mut World,
) {
    let queued = queue_scroll(params, world);
    respond_once_received(world, responder, queued);
}

fn queue_scroll(params: Option<Value>, world: &mut World) -> BrpResult<QueuedInputId> {
    can_receive_input(world)?;
    let BrpScrollParams {
        entity,
        window,
        position,
        delta,
        unit,
    } = parse_some(params)?;
    let target = pointer_target(entity, window, position)?;
    synthetic_input::scroll(world, target, unit, delta)
        .map_err(|err| invalid_params(err.to_string()))
}

fn pointer_target(
    entity: Option<Entity>,
    window: Option<Entity>,
    position: Option<Vec2>,
) -> BrpResult<ClickTarget> {
    match (entity, window, position) {
        (Some(entity), None, None) => Ok(ClickTarget::Node(entity)),
        (None, Some(window), Some(position)) => Ok(ClickTarget::Point { window, position }),
        _ => Err(invalid_params(
            "mouse input takes an `entity`, or a `window` and a `position`".into(),
        )),
    }
}

/// What to do with a button given an optional `state` and `hold`, or `None` given both.
fn button_action(state: Option<ButtonState>, hold: Option<u32>) -> Option<ButtonAction> {
    match (state, hold) {
        (None, hold) => Some(ButtonAction::Tap {
            hold: hold.unwrap_or(1),
        }),
        (Some(ButtonState::Pressed), None) => Some(ButtonAction::Press),
        (Some(ButtonState::Released), None) => Some(ButtonAction::Release),
        (Some(_), Some(_)) => None,
    }
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
    let action = button_action(state, hold)
        .ok_or_else(|| invalid_params("a key takes a `state` or a `hold`, not both".into()))?;
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
    fn a_drag_is_a_press_a_move_and_a_release() {
        let (mut world, window) = world();
        let at = |x: f32| json!({ "window": window, "position": [x, 6.0] });
        let with = |mut params: Value, extra: Value| {
            params
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            params
        };

        let pressed = with(at(5.0), json!({ "state": "Pressed" }));
        let pressed = send(&mut world, process_remote_click_request, pressed);
        let moved = with(at(50.0), json!({ "frames": 3, "ease": "QuadraticOut" }));
        let moved = send(&mut world, process_remote_move_cursor_request, moved);
        let released = with(at(50.0), json!({ "state": "Released" }));
        let released = send(&mut world, process_remote_click_request, released);
        let scrolled = with(at(5.0), json!({ "delta": [0.0, -120.0], "unit": "Pixel" }));
        let scrolled = send(&mut world, process_remote_scroll_request, scrolled);

        for (request, frame) in [(pressed, 11), (moved, 14), (released, 16), (scrolled, 18)] {
            let (answer, ran) = answered(&mut world, request);
            assert_eq!((answer.unwrap(), ran), (Value::Null, frame));
        }

        // The queue has drained, and a later move is still answered after its last frame.
        let again = with(at(5.0), json!({ "frames": 2 }));
        let again = send(&mut world, process_remote_move_cursor_request, again);
        let (answer, ran) = answered(&mut world, again);
        assert_eq!((answer.unwrap(), ran), (Value::Null, 21));
    }

    #[test]
    fn mouse_input_needs_one_target_and_a_bounded_move() {
        let (mut world, window) = world();
        let at = json!({ "window": window, "position": [5.0, 6.0] });
        let with = |extra: Value| {
            let mut params = at.clone();
            params
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            params
        };

        let both = with(json!({ "state": "Pressed", "hold": 2 }));
        assert_eq!(
            click(&mut world, both).unwrap_err().code,
            error_codes::INVALID_PARAMS
        );
        for params in [
            json!({ "window": window }),
            with(json!({ "frames": 0 })),
            with(json!({ "frames": 256 })),
            with(json!({ "ease": "Wobbly" })),
        ] {
            let err = queue_move_cursor(Some(params.clone()), &mut world).unwrap_err();
            assert_eq!(err.code, error_codes::INVALID_PARAMS, "{params}");
        }
        let err = queue_scroll(Some(at.clone()), &mut world).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
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
