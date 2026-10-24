//! Controls pausing and stepping the systems of the remote app with the `stepping.*` methods.
//!
//! The remote app needs the `bevy_debug_stepping` feature and a `Stepping` resource. The controls
//! are hidden once the remote app reports that either is missing.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::time::Duration;

use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_feathers::{
    constants::{fonts, size},
    controls::FeathersButton,
    display::caption,
    font_styles::InheritableFont,
};
use bevy_log::{info, warn};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_remote::{client::BrpClientError, error_codes, BrpError};
use bevy_scene::{bsn, on, Scene};
use bevy_tasks::{block_on, poll_once, Task};
use bevy_text::{FontSourceTemplate, FontWeight};
use bevy_time::{Real, Time};
use bevy_ui::{px, widget::Text, AlignItems, Display, FlexDirection, InteractionDisabled, Node};
use bevy_ui_widgets::Activate;
use serde_json::Value;

use super::{source::REQUEST_TIMEOUT, RemoteConnection, RemoteConnectionState, RemoteSource};
use crate::entity_tree::InspectorUi;

/// The time between two `stepping.status` polls.
const STATUS_INTERVAL: Duration = Duration::from_secs(1);
/// The number of button clicks kept while a `stepping.*` call is in flight.
const MAX_QUEUED: usize = 8;

/// One of the `stepping.*` methods.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum SteppingCall {
    /// Reads the stepping state of the remote app.
    #[default]
    Status,
    /// Pauses the remote app at the start of its stepping frame.
    Enable,
    /// Resumes the remote app.
    Disable,
    /// Runs the next system of the paused remote app.
    StepFrame,
    /// Runs the rest of the stepping frame of the paused remote app.
    ContinueFrame,
}

impl SteppingCall {
    /// The Bevy Remote Protocol method this call sends.
    pub fn method(self) -> &'static str {
        match self {
            SteppingCall::Status => "stepping.status",
            SteppingCall::Enable => "stepping.enable",
            SteppingCall::Disable => "stepping.disable",
            SteppingCall::StepFrame => "stepping.step_frame",
            SteppingCall::ContinueFrame => "stepping.continue_frame",
        }
    }
}

/// The stepping state of the remote app, as last reported by a `stepping.*` call.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub enum SteppingState {
    /// No `stepping.*` call has been answered yet.
    #[default]
    Unknown,
    /// The remote app has no `stepping.*` methods, or no `Stepping` resource.
    Unavailable,
    /// The remote app reported its stepping state.
    Known {
        /// Whether the remote app is paused for stepping.
        enabled: bool,
        /// The schedule and system the stepping cursor sits at, if any.
        cursor: Option<(String, String)>,
    },
}

struct SteppingTask {
    call: SteppingCall,
    task: Task<Result<Value, BrpClientError>>,
    started: Duration,
}

/// The `stepping.*` calls to the remote app, sent one at a time beside the entity polls.
#[derive(Resource, Default)]
pub struct RemoteStepping {
    source: Option<RemoteSource>,
    pending: Option<SteppingTask>,
    queue: Vec<SteppingCall>,
    next_status: Duration,
}

/// Marker for the caption showing the stepping cursor.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct SteppingCursorLabel;

/// A button sending one `stepping.*` call.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct SteppingButton(pub SteppingCall);

/// Buttons pausing, stepping and resuming the remote app, and the position of its stepping cursor.
pub fn stepping_controls() -> impl Scene {
    bsn! {
        InspectorUi
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
        }
        InheritableFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            @stepping_button(SteppingCall::Enable, "Pause")
            --
            @stepping_button(SteppingCall::StepFrame, "Step")
            --
            @stepping_button(SteppingCall::ContinueFrame, "Continue")
            --
            @stepping_button(SteppingCall::Disable, "Resume")
            --
            SteppingCursorLabel
            @caption("")
        ]
    }
}

fn stepping_button(call: SteppingCall, label: &'static str) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { @caption(label) },
        }
        SteppingButton({call})
        InteractionDisabled
        on(stepping_button_clicked)
    }
}

/// Observer queuing the `stepping.*` call of a clicked [`SteppingButton`].
pub fn stepping_button_clicked(
    activate: On<Activate>,
    buttons: Query<&SteppingButton>,
    mut stepping: ResMut<RemoteStepping>,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    if stepping.queue.len() < MAX_QUEUED {
        stepping.queue.push(button.0);
    }
}

/// Sends the queued `stepping.*` calls, and polls `stepping.status` while connected.
pub fn poll_remote_stepping(
    time: Option<Res<Time<Real>>>,
    connection: Res<RemoteConnection>,
    mut stepping: ResMut<RemoteStepping>,
    mut state: ResMut<SteppingState>,
) {
    let now = time.map(|time| time.elapsed()).unwrap_or_default();
    let stepping = &mut *stepping;
    let connected = matches!(connection.state, RemoteConnectionState::Connected { .. });
    if stepping.source.as_ref() != connection.source() || !connected {
        stepping.source = connection.source().cloned();
        stepping.pending = None;
        stepping.queue.clear();
        stepping.next_status = Duration::ZERO;
        state.set_if_neq(SteppingState::Unknown);
    }
    let Some(client) = connection.client().filter(|_| connected) else {
        return;
    };

    if let Some(mut pending) = stepping.pending.take() {
        match block_on(poll_once(&mut pending.task)) {
            Some(result) => finish_stepping_call(stepping, &mut state, pending.call, result),
            None if now.saturating_sub(pending.started) >= REQUEST_TIMEOUT => {
                warn!("the remote app did not answer `{}`", pending.call.method());
            }
            None => {
                stepping.pending = Some(pending);
                return;
            }
        }
    }

    if *state == SteppingState::Unavailable {
        stepping.queue.clear();
        return;
    }
    let call = if !stepping.queue.is_empty() {
        stepping.queue.remove(0)
    } else if now >= stepping.next_status {
        stepping.next_status = now + STATUS_INTERVAL;
        SteppingCall::Status
    } else {
        return;
    };
    stepping.pending = Some(SteppingTask {
        call,
        task: client.spawn_call(call.method(), None),
        started: now,
    });
}

fn finish_stepping_call(
    stepping: &mut RemoteStepping,
    state: &mut SteppingState,
    call: SteppingCall,
    result: Result<Value, BrpClientError>,
) {
    match result {
        Ok(value) => *state = parse_stepping_status(&value),
        Err(BrpClientError::Remote(BrpError { code, message, .. }))
            if code == error_codes::METHOD_NOT_FOUND || code == error_codes::RESOURCE_ERROR =>
        {
            info!("the remote app cannot be stepped: {message}");
            stepping.queue.clear();
            *state = SteppingState::Unavailable;
        }
        Err(error) => warn!("the remote app rejected `{}`: {error}", call.method()),
    }
}

fn parse_stepping_status(value: &Value) -> SteppingState {
    let cursor = value.get("cursor").and_then(|cursor| {
        Some((
            cursor.get("schedule")?.as_str()?.to_string(),
            cursor.get("system")?.as_str()?.to_string(),
        ))
    });
    SteppingState::Known {
        enabled: value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
        cursor,
    }
}

/// Shows the stepping buttons that apply to the current [`SteppingState`], and the cursor.
pub fn sync_stepping_controls(
    state: Res<SteppingState>,
    mut buttons: Query<(Entity, &SteppingButton, &mut Node, Has<InteractionDisabled>)>,
    mut labels: Query<&mut Text, With<SteppingCursorLabel>>,
    mut commands: Commands,
) {
    for (entity, button, mut node, disabled) in &mut buttons {
        let (shown, active) = match (&*state, button.0) {
            (SteppingState::Unavailable, _) => (false, false),
            (SteppingState::Unknown, call) => (call != SteppingCall::Disable, false),
            (SteppingState::Known { enabled, .. }, SteppingCall::Enable) => (!enabled, true),
            (SteppingState::Known { enabled, .. }, _) => (*enabled, true),
        };
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        if active == disabled {
            if active {
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
    }

    let cursor = match &*state {
        SteppingState::Known {
            cursor: Some((schedule, system)),
            ..
        } => format!("{schedule} / {system}"),
        SteppingState::Unavailable => "Stepping unavailable".to_string(),
        _ => String::new(),
    };
    for mut text in &mut labels {
        if text.0 != cursor {
            text.0 = cursor.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_scene::WorldSceneExt;
    use serde_json::json;

    fn finish(state: &mut SteppingState, result: Result<Value, BrpClientError>) {
        let mut stepping = RemoteStepping::default();
        stepping.queue.push(SteppingCall::StepFrame);
        finish_stepping_call(&mut stepping, state, SteppingCall::Status, result);
    }

    #[test]
    fn reads_the_stepping_status() {
        let mut state = SteppingState::Unknown;
        finish(
            &mut state,
            Ok(json!({
                "enabled": true,
                "schedules": ["Update"],
                "cursor": { "schedule": "Update", "system": "demo::system" },
            })),
        );
        assert_eq!(
            state,
            SteppingState::Known {
                enabled: true,
                cursor: Some(("Update".to_string(), "demo::system".to_string())),
            }
        );

        finish(
            &mut state,
            Ok(json!({ "enabled": false, "schedules": [], "cursor": null })),
        );
        assert_eq!(
            state,
            SteppingState::Known {
                enabled: false,
                cursor: None,
            }
        );
    }

    #[test]
    fn a_missing_method_makes_stepping_unavailable() {
        let mut state = SteppingState::Unknown;
        finish(
            &mut state,
            Err(BrpClientError::Remote(BrpError {
                code: error_codes::METHOD_NOT_FOUND,
                message: "no method".to_string(),
                data: None,
            })),
        );
        assert_eq!(state, SteppingState::Unavailable);

        let mut state = SteppingState::Unknown;
        finish(
            &mut state,
            Err(BrpClientError::Remote(BrpError {
                code: error_codes::RESOURCE_ERROR,
                message: "no Stepping resource".to_string(),
                data: None,
            })),
        );
        assert_eq!(state, SteppingState::Unavailable);

        let mut state = SteppingState::Unknown;
        finish(
            &mut state,
            Err(BrpClientError::InvalidResponse("dropped".to_string())),
        );
        assert_eq!(state, SteppingState::Unknown);
    }

    #[test]
    fn buttons_follow_the_stepping_state() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.init_resource::<SteppingState>();
        app.add_systems(bevy_app::Update, sync_stepping_controls);
        app.world_mut().spawn_scene(stepping_controls()).unwrap();

        let rows = |app: &mut App| {
            let mut query = app
                .world_mut()
                .query::<(&SteppingButton, &Node, Has<InteractionDisabled>)>();
            let mut rows: Vec<(SteppingCall, bool, bool)> = query
                .iter(app.world())
                .map(|(button, node, disabled)| {
                    (button.0, node.display != Display::None, !disabled)
                })
                .collect();
            rows.sort_by_key(|(call, ..)| format!("{call:?}"));
            rows
        };

        app.update();
        assert_eq!(
            rows(&mut app),
            [
                (SteppingCall::ContinueFrame, true, false),
                (SteppingCall::Disable, false, false),
                (SteppingCall::Enable, true, false),
                (SteppingCall::StepFrame, true, false),
            ]
        );

        app.insert_resource(SteppingState::Known {
            enabled: true,
            cursor: Some(("Update".to_string(), "demo::system".to_string())),
        });
        app.update();
        assert_eq!(
            rows(&mut app),
            [
                (SteppingCall::ContinueFrame, true, true),
                (SteppingCall::Disable, true, true),
                (SteppingCall::Enable, false, true),
                (SteppingCall::StepFrame, true, true),
            ]
        );
        let mut labels = app
            .world_mut()
            .query_filtered::<&Text, With<SteppingCursorLabel>>();
        assert_eq!(
            labels.single(app.world()).unwrap().0,
            "Update / demo::system"
        );

        app.insert_resource(SteppingState::Unavailable);
        app.update();
        assert!(rows(&mut app).iter().all(|(_, shown, _)| !shown));
    }
}
