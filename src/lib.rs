#![feature(min_specialization)]
pub mod prelude;

pub use bevy_app as app;
pub use bevy_asset as asset;
pub use bevy_core as core;
pub use bevy_diagnostic as diagnostic;
pub use bevy_input as input;
pub use bevy_render as render;
pub use bevy_serialization as serialization;
pub use bevy_transform as transform;
pub use bevy_ui as ui;
pub use bevy_window as window;

pub use glam as math;
pub use legion;

use app::AppBuilder;

pub trait AddDefaultPlugins {
    fn add_default_plugins(&mut self) -> &mut Self;
}

impl AddDefaultPlugins for AppBuilder {
    fn add_default_plugins(&mut self) -> &mut Self {
        self
            .add_plugin(bevy_core::CorePlugin::default())
            .add_plugin(bevy_input::InputPlugin::default())
            .add_plugin(bevy_window::WindowPlugin::default())
            .add_plugin(bevy_render::RenderPlugin::default())
            .add_plugin(ui::UiPlugin::default());

        #[cfg(feature = "bevy_winit")]
        {
            self.add_plugin(bevy_winit::WinitPlugin::default());
        }
        #[cfg(not(feature = "bevy_winit"))]
        {
            self.add_plugin(bevy_app::schedule_run::ScheduleRunner::default());
        }

        #[cfg(feature = "bevy_wgpu")]
        {
            self.add_plugin(
                bevy_wgpu::WgpuRendererPlugin::default(),
            );
        }

        self
    }
    
}
