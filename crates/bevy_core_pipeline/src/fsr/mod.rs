mod fsr_manager;
mod util;

use self::fsr_manager::FsrManager;
use bevy_app::{App, Plugin};
use bevy_render::{renderer::RenderDevice, RenderApp};

pub struct FsrPlugin;

impl Plugin for FsrPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>().clone();

        app.sub_app_mut(RenderApp).insert_resource(
            FsrManager::new(render_device).expect("Failed to initialize FsrPlugin"),
        );
    }
}
