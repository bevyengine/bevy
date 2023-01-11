use bevy_app::{App, Plugin};
use bevy_ecs::system::Resource;
use bevy_render::{renderer::RenderDevice, RenderApp};
use fsr2_wgpu::{FfxDimensions2D, Fsr2Context, Fsr2InitializationFlags};

pub struct Fsr2Plugin;

impl Plugin for Fsr2Plugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        let render_device = render_app.world.resource::<RenderDevice>();

        let fsr_context = Fsr2Context::new(
            render_device.wgpu_device(),
            FfxDimensions2D {
                width: 1920,
                height: 1080,
            },
            FfxDimensions2D {
                width: 1920 * 4,
                height: 1080 * 4,
            },
            Fsr2InitializationFlags::AUTO_EXPOSURE
                | Fsr2InitializationFlags::INFINITE_DEPTH
                | Fsr2InitializationFlags::INVERTED_DEPTH,
        );

        render_app.insert_resource(Fsr2ContextWrapper(fsr_context));
    }
}

#[derive(Resource)]
struct Fsr2ContextWrapper(Fsr2Context);
