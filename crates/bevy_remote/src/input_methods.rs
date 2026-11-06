//! `input.*` verbs for the Bevy Remote Protocol.

use bevy_curve::EaseFunction;
use bevy_dev_tools::synthetic_input::{self, ButtonAction, ClickTarget};
use bevy_ecs::{entity::Entity, system::In, world::World};
use bevy_input::{
    keyboard::{Key, KeyCode},
    mouse::{MouseButton, MouseScrollUnit},
    ButtonState,
};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{builtin_methods::parse_some, error_codes, BrpError, BrpResult};

/// The method path for an `input.click` request.
pub const BRP_CLICK_METHOD: &str = "input.click";

/// `input.click`: Clicks on a UI node, or at a point in a window, or with `state`, only presses or
/// only releases the button there.
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

    /// Only press the button, or only release it. When omitted, the button is pressed and
    /// released.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ButtonState>,

    /// How many frames the button is held down for, when it is pressed and released. Defaults
    /// to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<u32>,
}

fn left() -> MouseButton {
    MouseButton::Left
}

/// The response to an `input.click` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpClickResponse {
    /// The frame on which the app receives the button's last event, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// Handles an `input.click` request coming from a client.
pub fn process_remote_click_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
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
    let frame = synthetic_input::click(world, target, button, action)
        .map_err(|err| invalid_params(err.to_string()))?;
    serde_json::to_value(BrpClickResponse { frame }).map_err(BrpError::internal)
}

/// The method path for an `input.move` request.
pub const BRP_MOVE_METHOD: &str = "input.move";

/// The method path for an `input.scroll` request.
pub const BRP_SCROLL_METHOD: &str = "input.scroll";

/// `input.move`: Moves the cursor to a UI node, or to a point in a window, without pressing
/// anything, over `frames` frames eased by `ease`.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with a [`BrpMoveResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpMoveParams {
    /// The UI node to move to the center of. A node hidden by a scrolled ancestor is scrolled
    /// into view first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to move in, with `position`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to move to in `window`, in physical pixels from its top left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// How many frames the move takes, one event per frame. Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u32>,

    /// How the cursor speeds up and slows down along the way. Defaults to
    /// [`EaseFunction::Linear`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease: Option<EaseFunction>,
}

/// The response to an `input.move` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpMoveResponse {
    /// The frame on which the app receives the cursor's last move, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// `input.scroll`: Turns the mouse wheel over a UI node, or over a point in a window.
///
/// Give either `entity`, or both `window` and `position`.
///
/// The server responds with a [`BrpScrollResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BrpScrollParams {
    /// The UI node to scroll over the center of. A node hidden by a scrolled ancestor is scrolled
    /// into view first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,

    /// The window to scroll in, with `position`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Entity>,

    /// The point to scroll over in `window`, in physical pixels from its top left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec2>,

    /// How far the wheel turns, horizontally and vertically, in `unit`s. A positive `y` scrolls
    /// up.
    pub delta: Vec2,

    /// What `delta` counts. Defaults to [`MouseScrollUnit::Line`].
    #[serde(default = "line")]
    pub unit: MouseScrollUnit,
}

fn line() -> MouseScrollUnit {
    MouseScrollUnit::Line
}

/// The response to an `input.scroll` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpScrollResponse {
    /// The frame on which the app receives the wheel's turn, as counted by
    /// [`FrameCount`](bevy_diagnostic::FrameCount).
    pub frame: u32,
}

/// Handles an `input.move` request coming from a client.
pub fn process_remote_move_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpMoveParams {
        entity,
        window,
        position,
        frames,
        ease,
    } = parse_some(params)?;
    let target = pointer_target(entity, window, position)?;
    let frames = frames.unwrap_or(1);
    let ease = ease.unwrap_or(EaseFunction::Linear);
    let frame = synthetic_input::move_cursor(world, target, frames, ease)
        .map_err(|err| invalid_params(err.to_string()))?;
    serde_json::to_value(BrpMoveResponse { frame }).map_err(BrpError::internal)
}

/// Handles an `input.scroll` request coming from a client.
pub fn process_remote_scroll_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpScrollParams {
        entity,
        window,
        position,
        delta,
        unit,
    } = parse_some(params)?;
    let target = pointer_target(entity, window, position)?;
    let frame = synthetic_input::scroll(world, target, unit, delta)
        .map_err(|err| invalid_params(err.to_string()))?;
    serde_json::to_value(BrpScrollResponse { frame }).map_err(BrpError::internal)
}

fn pointer_target(
    entity: Option<Entity>,
    window: Option<Entity>,
    position: Option<Vec2>,
) -> Result<ClickTarget, BrpError> {
    match (entity, window, position) {
        (Some(entity), None, None) => Ok(ClickTarget::Node(entity)),
        (None, Some(window), Some(position)) => Ok(ClickTarget::Point { window, position }),
        _ => Err(invalid_params(
            "pointer input takes an `entity`, or a `window` and a `position`".into(),
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

/// Handles an `input.key` request coming from a client.
pub fn process_remote_key_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
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
    let frame = synthetic_input::key(
        world,
        window,
        key_code,
        logical_key,
        text.as_deref(),
        action,
    )
    .map_err(|err| invalid_params(err.to_string()))?;
    serde_json::to_value(BrpKeyResponse { frame }).map_err(BrpError::internal)
}

/// Handles an `input.type` request coming from a client.
pub fn process_remote_type_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpTypeParams { text, window } = parse_some(params)?;
    let frame = synthetic_input::type_text(world, window, &text)
        .map_err(|err| invalid_params(err.to_string()))?;
    serde_json::to_value(BrpTypeResponse { frame }).map_err(BrpError::internal)
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
        let window = world.spawn((Window::default(), PrimaryWindow)).id();
        (world, window)
    }

    fn click(world: &mut World, params: Value) -> BrpResult {
        process_remote_click_request(In(Some(params)), world)
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

        let pressed = click(&mut world, with(at(5.0), json!({ "state": "Pressed" })));
        assert_eq!(pressed.unwrap(), json!({ "frame": 11 }));
        let moved = with(at(50.0), json!({ "frames": 3, "ease": "QuadraticOut" }));
        let moved = process_remote_move_request(In(Some(moved)), &mut world);
        assert_eq!(moved.unwrap(), json!({ "frame": 14 }));
        let released = click(&mut world, with(at(50.0), json!({ "state": "Released" })));
        assert_eq!(released.unwrap(), json!({ "frame": 16 }));
        let scroll = with(at(5.0), json!({ "delta": [0.0, -120.0], "unit": "Pixel" }));
        let scrolled = process_remote_scroll_request(In(Some(scroll)), &mut world);
        assert_eq!(scrolled.unwrap(), json!({ "frame": 18 }));

        let both = with(at(5.0), json!({ "state": "Pressed", "hold": 2 }));
        assert_eq!(
            click(&mut world, both).unwrap_err().code,
            error_codes::INVALID_PARAMS
        );
        for params in [
            json!({ "window": window }),
            with(at(5.0), json!({ "frames": 0 })),
            with(at(5.0), json!({ "ease": "Wobbly" })),
        ] {
            let err =
                process_remote_move_request(In(Some(params.clone())), &mut world).unwrap_err();
            assert_eq!(err.code, error_codes::INVALID_PARAMS, "{params}");
        }
        let err = process_remote_scroll_request(In(Some(at(5.0))), &mut world).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
    }

    #[test]
    fn keys_and_text_answer_with_the_frame_of_their_last_event() {
        let (mut world, _) = world();
        let key = |world: &mut World, params| process_remote_key_request(In(Some(params)), world);

        let tapped = key(&mut world, json!({ "key_code": "KeyA" }));
        assert_eq!(tapped.unwrap(), json!({ "frame": 11 }));
        let pressed = key(
            &mut world,
            json!({ "key_code": "ShiftLeft", "state": "Pressed" }),
        );
        assert_eq!(pressed.unwrap(), json!({ "frame": 12 }));
        let typed = process_remote_type_request(In(Some(json!({ "text": "a" }))), &mut world);
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
            let err = process_remote_key_request(In(Some(params.clone())), &mut world).unwrap_err();
            assert_eq!(err.code, error_codes::INVALID_PARAMS, "{params}");
        }
        let err =
            process_remote_type_request(In(Some(json!({ "text": "" }))), &mut world).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
    }
}
