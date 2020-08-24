mod event;
mod system;
mod window;
mod windows;

pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

pub mod prelude {
    pub use crate::{CursorMoved, Window, WindowDescriptor, Windows};
}

use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

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
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowCloseRequested>()
            .add_event::<CloseWindow>()
            .add_event::<CursorMoved>()
            .init_resource::<Windows>();

        if self.add_primary_window {
            let resources = app.resources();
            let window_descriptor = resources
                .get::<WindowDescriptor>()
                .map(|descriptor| (*descriptor).clone())
                .unwrap_or_else(WindowDescriptor::default);
            let mut create_window_event = resources.get_mut::<Events<CreateWindow>>().unwrap();
            create_window_event.send(CreateWindow {
                id: WindowId::primary(),
                descriptor: window_descriptor,
            });
        }

        if self.exit_on_close {
            app.add_system(exit_on_window_close_system.system());
        }
    }
}
