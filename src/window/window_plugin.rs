use super::{CreateWindow, WindowCreated, WindowResized, Windows, WindowDescriptor};
use crate::{core::Events, app::{plugin::AppPlugin, AppBuilder}};

pub struct WindowPlugin {
    pub primary_window: Option<WindowDescriptor>,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(WindowDescriptor::default()),
        }
    }
}

impl AppPlugin for WindowPlugin {
    fn build(&self, mut app: AppBuilder) -> AppBuilder {
        app = app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_resource(Windows::default());

        if let Some(ref primary_window_descriptor) = self.primary_window {
            let mut create_window_event = app.resources.get_mut::<Events<CreateWindow>>().unwrap();
            create_window_event.send(CreateWindow {
                descriptor: primary_window_descriptor.clone(), 
            });
        }

        app
    }

    fn name(&self) -> &'static str {
        "Window"
    }
}
