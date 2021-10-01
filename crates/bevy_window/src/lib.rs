mod event;
mod raw_window_handle;
mod system;
mod window;
mod windows;

pub use crate::raw_window_handle::*;
pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, ReceivedCharacter, Window,
        WindowDescriptor, WindowMoved, Windows,
    };
}

use bevy_app::{prelude::*, Events};
use bevy_ecs::system::IntoSystem;

pub struct WindowPlugin {
    /// Whether to add a default window based on the [`WindowDescriptor`] resource
    pub add_primary_window: bool,
    /// Whether to close the app when there are no open windows
    pub exit_on_all_closed: bool,
    /// Whether to close windows when they are requested to be closed (i.e. when the close button is pressed)
    pub close_when_requested: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            close_when_requested: true,
            exit_on_all_closed: true,
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
                .unwrap_or_else(WindowDescriptor::default);
            let mut create_window_event = app
                .world
                .get_resource_mut::<Events<CreateWindow>>()
                .unwrap();
            create_window_event.send(CreateWindow {
                id: WindowId::primary(),
                descriptor: window_descriptor,
            });
        }

        if self.exit_on_all_closed {
            app.add_system(exit_on_all_closed.system());
        }
        if self.close_when_requested {
            app.add_system(close_when_requested.system());
        }
    }
}
