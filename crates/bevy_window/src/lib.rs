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

use bevy_app::{prelude::*, Events};

pub struct WindowPlugin {
    pub add_primary_window: bool,
    pub exit_on_close: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            exit_on_close: true,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowCloseRequested>()
            .add_event::<CloseWindow>()
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
            let mut create_window_event = app
                .world
                .get_resource_mut::<Events<CreateWindow>>()
                .unwrap();
            create_window_event.send(CreateWindow {
                id: WindowId::primary(),
                descriptor: window_descriptor,
            });
        }

        if self.exit_on_close {
            app.add_system(exit_on_window_close_system);
        }
    }
}
