//! `input.*` verbs for the Bevy Remote Protocol.

use bevy_dev_tools::synthetic_input::{self, ClickTarget};
use bevy_ecs::{entity::Entity, system::In, world::World};
use bevy_input::mouse::MouseButton;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{builtin_methods::parse_some, error_codes, BrpError, BrpResult};

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
    /// The frame that reads the button's release, as counted by
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
    serde_json::to_value(BrpClickResponse { frame }).map_err(BrpError::internal)
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
}
