//! Controls pausing and stepping the systems of the remote app with the `stepping.*` methods.
//!
//! The remote app needs the `bevy_debug_stepping` feature and a `Stepping` resource. The controls
//! are disabled while the remote app reports that either is missing.

use alloc::{format, string::String};
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
use bevy_remote::{
    client::BrpClientError,
    error_codes,
    stepping_methods::{
        BrpSteppingCursor, BrpSteppingStatusResponse, BRP_STEPPING_CONTINUE_FRAME,
        BRP_STEPPING_DISABLE, BRP_STEPPING_ENABLE, BRP_STEPPING_STATUS, BRP_STEPPING_STEP_FRAME,
    },
    BrpError,
};
use bevy_scene::{bsn, on, Scene};
use bevy_tasks::{block_on, poll_once, Task};
use bevy_text::{FontSourceTemplate, FontWeight};
use bevy_time::{Real, Time};
use bevy_ui::{px, widget::Text, AlignItems, FlexDirection, InteractionDisabled, Node};
use bevy_ui_widgets::Activate;
use serde_json::Value;

use super::{source::REQUEST_TIMEOUT, RemoteConnection, RemoteConnectionState, RemoteSource};
use crate::entity_tree::InspectorUi;

/// The time between two `stepping.status` polls.
const STATUS_INTERVAL: Duration = Duration::from_secs(1);

/// One of the [`stepping.*`](bevy_remote::stepping_methods) methods.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum SteppingCall {
    /// Reads the stepping status of the remote app.
    #[default]
    Status,
    /// Pauses the remote app at the start of its stepping frame.
    Pause,
    /// Resumes the remote app.
    Resume,
    /// Runs the next system of the paused remote app.
    StepFrame,
    /// Runs the rest of the stepping frame of the paused remote app.
    ContinueFrame,
}

impl SteppingCall {
    /// The Bevy Remote Protocol method this call sends.
    pub fn method(self) -> &'static str {
        match self {
            SteppingCall::Status => BRP_STEPPING_STATUS,
            SteppingCall::Pause => BRP_STEPPING_ENABLE,
            SteppingCall::Resume => BRP_STEPPING_DISABLE,
            SteppingCall::StepFrame => BRP_STEPPING_STEP_FRAME,
            SteppingCall::ContinueFrame => BRP_STEPPING_CONTINUE_FRAME,
        }
    }
}

/// The stepping status of the remote app, as last reported by `stepping.status`.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub enum SteppingStatus {
    /// No `stepping.*` call has been answered yet.
    #[default]
    Unknown,
    /// The remote app has no `stepping.*` methods, or no `Stepping` resource.
    Unavailable,
    /// The remote app reported its stepping status.
    Known {
        /// Whether the remote app is paused for stepping.
        paused: bool,
        /// The schedule and system the stepping cursor sits at, if any.
        cursor: Option<BrpSteppingCursor>,
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
    requested: Option<SteppingCall>,
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
            @stepping_button(SteppingCall::Pause, "Pause")
            --
            @stepping_button(SteppingCall::StepFrame, "Step")
            --
            @stepping_button(SteppingCall::ContinueFrame, "Continue")
            --
            @stepping_button(SteppingCall::Resume, "Resume")
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

/// Observer requesting the `stepping.*` call of a clicked [`SteppingButton`].
///
/// A click while another call is in flight replaces any earlier request that has not been sent.
pub fn stepping_button_clicked(
    activate: On<Activate>,
    buttons: Query<&SteppingButton>,
    mut stepping: ResMut<RemoteStepping>,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    stepping.requested = Some(button.0);
}

/// Sends the requested `stepping.*` call, and polls `stepping.status` while connected.
///
/// A successful call other than `stepping.status` is followed by a `stepping.status` read.
///
/// Polling stops once the remote app reports that stepping is unavailable, since it cannot gain
/// the feature without restarting, and resumes when the connection changes or drops.
pub fn poll_remote_stepping(
    time: Option<Res<Time<Real>>>,
    connection: Res<RemoteConnection>,
    mut stepping: ResMut<RemoteStepping>,
    mut status: ResMut<SteppingStatus>,
) {
    let now = time.map(|time| time.elapsed()).unwrap_or_default();
    let stepping = &mut *stepping;
    let connected = matches!(connection.state, RemoteConnectionState::Connected { .. });
    if stepping.source.as_ref() != connection.source() || !connected {
        stepping.source = connection.source().cloned();
        stepping.pending = None;
        stepping.requested = None;
        stepping.next_status = Duration::ZERO;
        status.set_if_neq(SteppingStatus::Unknown);
    }
    let Some(client) = connection.client().filter(|_| connected) else {
        return;
    };

    if let Some(mut pending) = stepping.pending.take() {
        match block_on(poll_once(&mut pending.task)) {
            Some(result) => {
                if finish_stepping_call(&mut status, pending.call, result) {
                    stepping.next_status = now;
                }
            }
            None if now.saturating_sub(pending.started) >= REQUEST_TIMEOUT => {
                warn!("the remote app did not answer `{}`", pending.call.method());
            }
            None => {
                stepping.pending = Some(pending);
                return;
            }
        }
    }

    if *status == SteppingStatus::Unavailable {
        stepping.requested = None;
        return;
    }
    let call = if let Some(call) = stepping.requested.take() {
        call
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

/// Applies the answer to `call`, returning whether `stepping.status` should be read right away.
///
/// Only `stepping.status` answers update `status`, since the other calls take effect on the next
/// frame of the remote app.
fn finish_stepping_call(
    status: &mut SteppingStatus,
    call: SteppingCall,
    result: Result<Value, BrpClientError>,
) -> bool {
    match result {
        Ok(_) if call != SteppingCall::Status => return true,
        Ok(value) => match serde_json::from_value::<BrpSteppingStatusResponse>(value) {
            Ok(response) => {
                *status = SteppingStatus::Known {
                    paused: response.enabled,
                    cursor: response.cursor,
                };
            }
            Err(error) => warn!(
                "the remote app sent an invalid `{}` response: {error}",
                call.method()
            ),
        },
        Err(BrpClientError::Remote(BrpError { code, message, .. }))
            if code == error_codes::METHOD_NOT_FOUND || code == error_codes::RESOURCE_ERROR =>
        {
            info!("the remote app cannot be stepped: {message}");
            *status = SteppingStatus::Unavailable;
        }
        Err(error) => warn!("the remote app rejected `{}`: {error}", call.method()),
    }
    false
}

/// Enables the stepping buttons that apply to the current [`SteppingStatus`], and shows the cursor.
pub fn sync_stepping_controls(
    status: Res<SteppingStatus>,
    buttons: Query<(Entity, &SteppingButton, Has<InteractionDisabled>)>,
    mut labels: Query<&mut Text, With<SteppingCursorLabel>>,
    mut commands: Commands,
) {
    for (entity, button, disabled) in &buttons {
        let active = match (&*status, button.0) {
            (SteppingStatus::Known { paused, .. }, SteppingCall::Pause) => !paused,
            (SteppingStatus::Known { paused, .. }, _) => *paused,
            _ => false,
        };
        if active == disabled {
            if active {
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
    }

    let cursor = match &*status {
        SteppingStatus::Known {
            cursor: Some(BrpSteppingCursor { schedule, system }),
            ..
        } => format!("{schedule} / {system}"),
        SteppingStatus::Unavailable => String::from("Stepping unavailable"),
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
    use alloc::{string::ToString, vec::Vec};
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_scene::WorldSceneExt;
    use serde_json::json;

    fn finish(status: &mut SteppingStatus, result: Result<Value, BrpClientError>) {
        assert!(!finish_stepping_call(status, SteppingCall::Status, result));
    }

    fn cursor() -> BrpSteppingCursor {
        BrpSteppingCursor {
            schedule: "Update".to_string(),
            system: "demo::system".to_string(),
        }
    }

    #[test]
    fn reads_the_stepping_status() {
        let mut status = SteppingStatus::Unknown;
        finish(
            &mut status,
            Ok(json!({
                "enabled": true,
                "schedules": ["Update"],
                "cursor": { "schedule": "Update", "system": "demo::system" },
            })),
        );
        assert_eq!(
            status,
            SteppingStatus::Known {
                paused: true,
                cursor: Some(cursor()),
            }
        );

        finish(
            &mut status,
            Ok(json!({ "enabled": false, "schedules": [], "cursor": null })),
        );
        assert_eq!(
            status,
            SteppingStatus::Known {
                paused: false,
                cursor: None,
            }
        );
    }

    #[test]
    fn actions_request_a_status_read() {
        let known = SteppingStatus::Known {
            paused: false,
            cursor: None,
        };
        for call in [
            SteppingCall::Pause,
            SteppingCall::Resume,
            SteppingCall::StepFrame,
            SteppingCall::ContinueFrame,
        ] {
            let mut status = known.clone();
            assert!(finish_stepping_call(&mut status, call, Ok(Value::Null)));
            assert_eq!(status, known);
        }

        let mut status = known.clone();
        let error = BrpClientError::Remote(BrpError {
            code: error_codes::RESOURCE_ERROR,
            message: "no Stepping resource".to_string(),
            data: None,
        });
        assert!(!finish_stepping_call(
            &mut status,
            SteppingCall::Pause,
            Err(error)
        ));
        assert_eq!(status, SteppingStatus::Unavailable);
    }

    #[test]
    fn a_missing_method_makes_stepping_unavailable() {
        let mut status = SteppingStatus::Unknown;
        finish(
            &mut status,
            Err(BrpClientError::Remote(BrpError {
                code: error_codes::METHOD_NOT_FOUND,
                message: "no method".to_string(),
                data: None,
            })),
        );
        assert_eq!(status, SteppingStatus::Unavailable);

        let mut status = SteppingStatus::Unknown;
        finish(
            &mut status,
            Err(BrpClientError::Remote(BrpError {
                code: error_codes::RESOURCE_ERROR,
                message: "no Stepping resource".to_string(),
                data: None,
            })),
        );
        assert_eq!(status, SteppingStatus::Unavailable);

        let mut status = SteppingStatus::Unknown;
        finish(
            &mut status,
            Err(BrpClientError::InvalidResponse("dropped".to_string())),
        );
        assert_eq!(status, SteppingStatus::Unknown);
    }

    #[test]
    fn buttons_follow_the_stepping_status() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.init_resource::<SteppingStatus>();
        app.add_systems(bevy_app::Update, sync_stepping_controls);
        app.world_mut().spawn_scene(stepping_controls()).unwrap();

        let active = |app: &mut App| {
            let mut query = app
                .world_mut()
                .query::<(&SteppingButton, Has<InteractionDisabled>)>();
            let mut active: Vec<SteppingCall> = query
                .iter(app.world())
                .filter(|(_, disabled)| !disabled)
                .map(|(button, _)| button.0)
                .collect();
            active.sort_by_key(|call| format!("{call:?}"));
            active
        };

        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SteppingButton>()
                .iter(app.world())
                .count(),
            4
        );
        assert!(active(&mut app).is_empty());

        app.insert_resource(SteppingStatus::Known {
            paused: false,
            cursor: None,
        });
        app.update();
        assert_eq!(active(&mut app), [SteppingCall::Pause]);

        app.insert_resource(SteppingStatus::Known {
            paused: true,
            cursor: Some(cursor()),
        });
        app.update();
        assert_eq!(
            active(&mut app),
            [
                SteppingCall::ContinueFrame,
                SteppingCall::Resume,
                SteppingCall::StepFrame,
            ]
        );
        let mut labels = app
            .world_mut()
            .query_filtered::<&Text, With<SteppingCursorLabel>>();
        assert_eq!(
            labels.single(app.world()).unwrap().0,
            "Update / demo::system"
        );

        app.insert_resource(SteppingStatus::Unavailable);
        app.update();
        assert!(active(&mut app).is_empty());
    }
}
