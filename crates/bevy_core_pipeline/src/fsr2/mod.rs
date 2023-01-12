use bevy_app::{App, Plugin};
use bevy_ecs::system::Resource;
use bevy_render::{renderer::RenderDevice, view::Msaa, RenderApp};
use bevy_winit::WinitWindows;
use fsr2_wgpu::{Fsr2Context, Fsr2InitializationFlags, Fsr2Resolution};

pub struct Fsr2Plugin {
    pub hdr: bool,
}

impl Plugin for Fsr2Plugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app_mut(RenderApp).is_err() {
            return;
        }

        app.insert_resource(Msaa { samples: 1 });

        let max_resolution = max_monitor_size(app);

        let mut initialization_flags = Fsr2InitializationFlags::AUTO_EXPOSURE
            | Fsr2InitializationFlags::INFINITE_DEPTH
            | Fsr2InitializationFlags::INVERTED_DEPTH;
        if self.hdr {
            initialization_flags |= Fsr2InitializationFlags::HIGH_DYNAMIC_RANGE;
        }

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        let render_device = render_app.world.resource::<RenderDevice>();

        let fsr_context = Fsr2Context::new(
            render_device.wgpu_device(),
            max_resolution,
            max_resolution,
            initialization_flags,
        );

        render_app.insert_resource(Fsr2ContextWrapper(fsr_context));
    }
}

#[derive(Resource)]
struct Fsr2ContextWrapper(Fsr2Context);

fn max_monitor_size(app: &App) -> Fsr2Resolution {
    let mut max_resolution = Fsr2Resolution {
        width: 0,
        height: 0,
    };

    for monitor in app
        .world
        .get_non_send_resource::<WinitWindows>()
        .unwrap()
        .windows
        .values()
        .next()
        .unwrap()
        .available_monitors()
    {
        let monitor_size = monitor.size();
        max_resolution.width = max_resolution.width.max(monitor_size.width);
        max_resolution.height = max_resolution.height.max(monitor_size.height);
    }

    max_resolution
}
