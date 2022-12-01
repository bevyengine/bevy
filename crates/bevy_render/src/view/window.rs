use crate::{
    render_resource::TextureView,
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    Extract, RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_utils::{tracing::debug, HashMap, HashSet};
use bevy_window::{
    CompositeAlphaMode, PresentMode, RawHandleWrapper, WindowClosed, WindowId, Windows,
};
use std::ops::{Deref, DerefMut};
use wgpu::TextureFormat;

/// Token to ensure a system runs on the main thread.
#[derive(Resource, Default)]
pub struct NonSendMarker;

pub struct WindowRenderPlugin;

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WindowSystem {
    Prepare,
}

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedWindows>()
                .init_resource::<WindowSurfaces>()
                .init_resource::<NonSendMarker>()
                .add_system_to_stage(RenderStage::Extract, extract_windows)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_windows.label(WindowSystem::Prepare),
                );
        }
    }
}

pub struct ExtractedWindow {
    pub id: WindowId,
    pub raw_handle: Option<RawHandleWrapper>,
    pub physical_width: u32,
    pub physical_height: u32,
    pub present_mode: PresentMode,
    pub swap_chain_texture: Option<TextureView>,
    pub swap_chain_texture_format: Option<TextureFormat>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
}

#[derive(Default, Resource)]
pub struct ExtractedWindows {
    pub windows: HashMap<WindowId, ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = HashMap<WindowId, ExtractedWindow>;

    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

fn extract_windows(
    mut extracted_windows: ResMut<ExtractedWindows>,
    mut closed: Extract<EventReader<WindowClosed>>,
    windows: Extract<Res<Windows>>,
) {
    for window in windows.iter() {
        let (new_width, new_height) = (
            window.physical_width().max(1),
            window.physical_height().max(1),
        );
        let new_present_mode = window.present_mode();

        let mut extracted_window =
            extracted_windows
                .entry(window.id())
                .or_insert(ExtractedWindow {
                    id: window.id(),
                    raw_handle: window.raw_handle(),
                    physical_width: new_width,
                    physical_height: new_height,
                    present_mode: window.present_mode(),
                    swap_chain_texture: None,
                    swap_chain_texture_format: None,
                    size_changed: false,
                    present_mode_changed: false,
                    alpha_mode: window.alpha_mode(),
                });

        // NOTE: Drop the swap chain frame here
        extracted_window.swap_chain_texture = None;
        extracted_window.size_changed = new_width != extracted_window.physical_width
            || new_height != extracted_window.physical_height;
        extracted_window.present_mode_changed = new_present_mode != extracted_window.present_mode;

        if extracted_window.size_changed {
            debug!(
                "Window size changed from {}x{} to {}x{}",
                extracted_window.physical_width,
                extracted_window.physical_height,
                new_width,
                new_height
            );
            extracted_window.physical_width = new_width;
            extracted_window.physical_height = new_height;
        }

        if extracted_window.present_mode_changed {
            debug!(
                "Window Present Mode changed from {:?} to {:?}",
                extracted_window.present_mode, new_present_mode
            );
            extracted_window.present_mode = new_present_mode;
        }
    }
    for closed_window in closed.iter() {
        extracted_windows.remove(&closed_window.id);
    }
}

struct SurfaceData {
    surface: wgpu::Surface,
    format: TextureFormat,
}

#[derive(Resource, Default)]
pub struct WindowSurfaces {
    surfaces: HashMap<WindowId, SurfaceData>,
    /// List of windows that we have already called the initial `configure_surface` for
    configured_windows: HashSet<WindowId>,
}

/// Creates and (re)configures window surfaces, and obtains a swapchain texture for rendering.
///
/// NOTE: `get_current_texture` in `prepare_windows` can take a long time if the GPU workload is
/// the performance bottleneck. This can be seen in profiles as multiple prepare-stage systems all
/// taking an unusually long time to complete, and all finishing at about the same time as the
/// `prepare_windows` system. Improvements in bevy are planned to avoid this happening when it
/// should not but it will still happen as it is easy for a user to create a large GPU workload
/// relative to the GPU performance and/or CPU workload.
/// This can be caused by many reasons, but several of them are:
/// - GPU workload is more than your current GPU can manage
/// - Error / performance bug in your custom shaders
/// - wgpu was unable to detect a proper GPU hardware-accelerated device given the chosen
///   [`Backends`](crate::settings::Backends), [`WgpuLimits`](crate::settings::WgpuLimits),
///   and/or [`WgpuFeatures`](crate::settings::WgpuFeatures). For example, on Windows currently
///   `DirectX 11` is not supported by wgpu 0.12 and so if your GPU/drivers do not support Vulkan,
///   it may be that a software renderer called "Microsoft Basic Render Driver" using `DirectX 12`
///   will be chosen and performance will be very poor. This is visible in a log message that is
///   output during renderer initialization. Future versions of wgpu will support `DirectX 11`, but
///   another alternative is to try to use [`ANGLE`](https://github.com/gfx-rs/wgpu#angle) and
///   [`Backends::GL`](crate::settings::Backends::GL) if your GPU/drivers support `OpenGL 4.3` / `OpenGL ES 3.0` or
///   later.
pub fn prepare_windows(
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread,
    // which is necessary for some OS s
    _marker: NonSend<NonSendMarker>,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_device: Res<RenderDevice>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
) {
    for window in windows
        .windows
        .values_mut()
        // value of raw_handle is only None in synthetic tests
        .filter(|x| x.raw_handle.is_some())
    {
        let window_surfaces = window_surfaces.deref_mut();
        let surface_data = window_surfaces
            .surfaces
            .entry(window.id)
            .or_insert_with(|| unsafe {
                // NOTE: On some OSes this MUST be called from the main thread.
                let surface = render_instance
                    .create_surface(&window.raw_handle.as_ref().unwrap().get_handle());
                let format = *surface
                    .get_supported_formats(&render_adapter)
                    .get(0)
                    .unwrap_or_else(|| {
                        panic!(
                            "No supported formats found for surface {:?} on adapter {:?}",
                            surface, render_adapter
                        )
                    });
                SurfaceData { surface, format }
            });

        let surface_configuration = wgpu::SurfaceConfiguration {
            format: surface_data.format,
            width: window.physical_width,
            height: window.physical_height,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            present_mode: match window.present_mode {
                PresentMode::Fifo => wgpu::PresentMode::Fifo,
                PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
                PresentMode::Immediate => wgpu::PresentMode::Immediate,
                PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
                PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
            },
            alpha_mode: match window.alpha_mode {
                CompositeAlphaMode::Auto => wgpu::CompositeAlphaMode::Auto,
                CompositeAlphaMode::Opaque => wgpu::CompositeAlphaMode::Opaque,
                CompositeAlphaMode::PreMultiplied => wgpu::CompositeAlphaMode::PreMultiplied,
                CompositeAlphaMode::PostMultiplied => wgpu::CompositeAlphaMode::PostMultiplied,
                CompositeAlphaMode::Inherit => wgpu::CompositeAlphaMode::Inherit,
            },
        };

        // A recurring issue is hitting `wgpu::SurfaceError::Timeout` on certain Linux
        // mesa driver implementations. This seems to be a quirk of some drivers.
        // We'd rather keep panicking when not on Linux mesa, because in those case,
        // the `Timeout` is still probably the symptom of a degraded unrecoverable
        // application state.
        // see https://github.com/bevyengine/bevy/pull/5957
        // and https://github.com/gfx-rs/wgpu/issues/1218
        #[cfg(target_os = "linux")]
        let may_erroneously_timeout = || {
            render_instance
                .enumerate_adapters(wgpu::Backends::VULKAN)
                .any(|adapter| {
                    let name = adapter.get_info().name;
                    name.starts_with("AMD") || name.starts_with("Intel")
                })
        };

        let not_already_configured = window_surfaces.configured_windows.insert(window.id);

        let surface = &surface_data.surface;
        if not_already_configured || window.size_changed || window.present_mode_changed {
            render_device.configure_surface(surface, &surface_configuration);
            let frame = surface
                .get_current_texture()
                .expect("Error configuring surface");
            window.swap_chain_texture = Some(TextureView::from(frame));
        } else {
            match surface.get_current_texture() {
                Ok(frame) => {
                    window.swap_chain_texture = Some(TextureView::from(frame));
                }
                Err(wgpu::SurfaceError::Outdated) => {
                    render_device.configure_surface(surface, &surface_configuration);
                    let frame = surface
                        .get_current_texture()
                        .expect("Error reconfiguring surface");
                    window.swap_chain_texture = Some(TextureView::from(frame));
                }
                #[cfg(target_os = "linux")]
                Err(wgpu::SurfaceError::Timeout) if may_erroneously_timeout() => {
                    bevy_utils::tracing::trace!(
                        "Couldn't get swap chain texture. This is probably a quirk \
                        of your Linux GPU driver, so it can be safely ignored."
                    );
                }
                Err(err) => {
                    panic!("Couldn't get swap chain texture, operation unrecoverable: {err}");
                }
            }
        };
        window.swap_chain_texture_format = Some(surface_data.format);
    }
}
