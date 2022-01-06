mod cursor;
mod event;
mod raw_window_handle;
mod system;
mod window;
mod windows;

pub use crate::raw_window_handle::*;
pub use cursor::*;
pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorIcon, CursorLeft, CursorMoved, FileDragAndDrop, ReceivedCharacter,
        Window, WindowDescriptor, WindowMoved, Windows,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::{event::Events, schedule::SystemLabel};

pub struct WindowPlugin {
    pub add_primary_window: bool,
    /// Whether to close the app when there are no open windows.
    /// If disabling this, consider ensuring that you send a [`bevy_app::AppExit`] event yourself
    /// when the app should exit; otherwise you will create headless processes, which would be
    /// surprising for your users.
    ///
    /// This setting controls whether this plugin adds [`exit_on_all_closed`]
    pub exit_on_all_closed: bool,
    /// Whether to close windows when they are requested to be closed (i.e. when the close button is pressed)
    ///
    /// This setting controls whether this plugin adds [`close_when_requested`]
    pub close_when_requested: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            exit_on_all_closed: true,
            close_when_requested: true,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowClosed>()
            .add_event::<WindowCloseRequested>()
            .add_event::<RequestRedraw>()
            .add_event::<CursorMoved>()
            .add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<ReceivedCharacter>()
            .add_event::<WindowFocused>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<WindowBackendScaleFactorChanged>()
            .add_event::<FileDragAndDrop>()
            .add_event::<WindowMoved>()
            .init_resource::<Windows>();

        if self.add_primary_window {
            let window_descriptor = app
                .world
                .get_resource::<WindowDescriptor>()
                .map(|descriptor| (*descriptor).clone())
                .unwrap_or_default();
            let mut create_window_event = app.world.resource_mut::<Events<CreateWindow>>();
            create_window_event.send(CreateWindow {
                id: WindowId::primary(),
                descriptor: window_descriptor,
            });
        }

        if self.exit_on_all_closed {
            app.add_system(exit_on_all_closed);
        }
        if self.close_when_requested {
            app.add_system(close_when_requested);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub struct ModifiesWindows;
