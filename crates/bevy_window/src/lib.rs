mod event;
mod system;
mod window;
mod windows;

pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

use bevy_app::{AppBuilder, AppPlugin, Events};

pub struct WindowPlugin {
    pub primary_window: Option<WindowDescriptor>,
    pub exit_on_close: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(WindowDescriptor::default()),
            exit_on_close: true,
        }
    }
}

impl AppPlugin for WindowPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowCloseRequested>()
            .add_event::<CloseWindow>()
            .add_event::<CursorMoved>()
            .init_resource::<Windows>();

        if let Some(ref primary_window_descriptor) = self.primary_window {
            let mut create_window_event =
                app.resources().get_mut::<Events<CreateWindow>>().unwrap();
            create_window_event.send(CreateWindow {
                descriptor: primary_window_descriptor.clone(),
            });
        }

        if self.exit_on_close {
            let exit_on_close_system = exit_on_window_close_system(None);
            app.add_system(exit_on_close_system);
        }
    }
}
