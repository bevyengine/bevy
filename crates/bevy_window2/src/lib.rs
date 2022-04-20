#![deny(missing_docs)]
//! Windows for the game engine

mod commands;
mod cursor;
mod events;
mod exit;
mod window;

pub use commands::*;
pub use cursor::*;
pub use events::*;
pub use exit::*;
pub use window::*;

use bevy_app::prelude::*;

/// Adds support for a Bevy Application to create windows
pub struct WindowPlugin {
    /// Create a primary window automatically
    ///
    /// If a [`WindowDescriptor`] resource is inserted before this plugin loaded,
    /// the primary window will use that descriptor
    pub add_primary_window: bool,
    /// Condition for app to exit
    pub exit_condition: ExitCondition,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            // should this default to OnAllClosed or OnPrimaryClosed
            exit_condition: ExitCondition::OnAllClosed,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<CursorMoved>()
            .add_event::<FileDragAndDrop>()
            .add_event::<ReceivedCharacter>()
            .add_event::<RequestRedraw>()
            .add_event::<WindowCloseRequested>()
            .add_event::<WindowFocused>()
            .add_event::<WindowMoved>()
            .add_event::<WindowResized>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<WindowScaleFactorBackendChanged>();

        if self.add_primary_window {
            let descriptor = app
                .world
                .get_resource::<WindowDescriptor>()
                .map(|descriptor| (*descriptor).clone())
                .unwrap_or_default();
            let window = app.world.spawn().insert(descriptor).id();
            app.insert_resource(PrimaryWindow(window));
        }

        match self.exit_condition {
            ExitCondition::OnAllClosed => {
                app.add_system(exit_on_all_window_closed_system);
            }
            ExitCondition::OnPrimaryClosed => {
                app.add_system(exit_on_primary_window_closed_system);
            }
            _ => {}
        }
    }
}
