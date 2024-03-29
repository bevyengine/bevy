mod fsr_manager;
mod util;

use self::fsr_manager::FsrManager;
use bevy_app::{App, Plugin};
use bevy_render::{renderer::RenderDevice, view::Msaa, RenderApp};

pub struct FsrPlugin;

impl Plugin for FsrPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
    }

    fn finish(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>().clone();

        app.sub_app_mut(RenderApp).insert_resource(
            FsrManager::new(render_device).expect("Failed to initialize FsrPlugin"),
        );
    }
}

pub enum FsrQualityMode {
    /// Upscale by 1.5x
    Quality,
    /// Upscale by 1.7x
    Balanced,
    /// Upscale by 2.0x
    Peformance,
    /// Upscale by 3.0x
    UltraPerformance,
}
