use crate::{PrimaryWindow, Window};

use bevy_app::AppExit;
use bevy_ecs::{
    event::EventWriter,
    system::{Query, Res},
};

/// Exit condition
pub enum ExitCondition {
    /// Exit app when all windows are closed
    OnAllClosed,
    /// Exit app when the primary window is closed
    OnPrimaryClosed,
    /// Stay headless even if all windows are closed
    DontExit,
}

/// system for [`ExitCondition::OnAllClosed`]
pub fn exit_on_all_window_closed_system(
    mut app_exit_events: EventWriter<AppExit>,
    windows: Query<&Window>,
) {
    if windows.is_empty() {
        app_exit_events.send(AppExit);
    }
}

/// system for [`ExitCondition::OnPrimaryClosed`]
pub fn exit_on_primary_window_closed_system(
    mut app_exit_events: EventWriter<AppExit>,
    windows: Query<&Window>,
    primary_window: Res<PrimaryWindow>,
) {
    if windows.get(**primary_window).is_err() {
        app_exit_events.send(AppExit);
    }
}
