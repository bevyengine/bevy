use super::{CreateWindow, Time, WindowCreated, WindowResized, Windows, Events, WindowDescriptor};
use crate::app::{plugin::AppPlugin, AppBuilder};
use bevy_transform::transform_system_bundle;

pub struct CorePlugin {
    pub primary_window: Option<WindowDescriptor>,
}

impl Default for CorePlugin {
    fn default() -> Self {
        CorePlugin {
            primary_window: Some(WindowDescriptor::default()),
        }
    }
}

impl AppPlugin for CorePlugin {
    fn build(&self, mut app: AppBuilder) -> AppBuilder {
        for transform_system in transform_system_bundle::build(&mut app.world).drain(..) {
            app = app.add_system(transform_system);
        }

        app = app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_resource(Windows::default())
            .add_resource(Time::new());

        if let Some(ref primary_window_descriptor) = self.primary_window {
            let mut create_window_event = app.resources.get_mut::<Events<CreateWindow>>().unwrap();
            create_window_event.send(CreateWindow {
                descriptor: primary_window_descriptor.clone(), 
            });
        }

        app
    }

    fn name(&self) -> &'static str {
        "Core"
    }
}
