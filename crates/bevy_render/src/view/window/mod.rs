use crate::renderer::WgpuWrapper;
use crate::{
    render_resource::{SurfaceTexture, TextureView},
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_log::{debug, info, warn};
use bevy_platform::collections::HashSet;
use bevy_utils::default;
use bevy_window::{
    CompositeAlphaMode, PresentMode, PrimaryWindow, RawHandleWrapper, Window, WindowClosing,
};
use core::{
    num::NonZero,
    ops::{Deref, DerefMut},
};
use wgpu::{
    SurfaceConfiguration, SurfaceTargetUnsafe, TextureFormat, TextureUsages, TextureViewDescriptor,
};

pub mod screenshot;

use screenshot::ScreenshotPlugin;

pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScreenshotPlugin);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedWindows>()
                .init_resource::<WindowSurfaces>()
                .add_systems(ExtractSchedule, extract_windows)
                .add_systems(
                    Render,
                    create_surfaces
                        .run_if(need_surface_configuration)
                        .before(prepare_windows),
                )
                .add_systems(Render, prepare_windows.in_set(RenderSystems::ManageViews));
        }
    }
}

pub struct ExtractedWindow {
    /// An entity that contains the components in [`Window`].
    pub entity: Entity,
    pub handle: RawHandleWrapper,
    pub physical_width: u32,
    pub physical_height: u32,
    pub present_mode: PresentMode,
    pub desired_maximum_frame_latency: Option<NonZero<u32>>,
    /// Note: this will not always be the swap chain texture view. When taking a screenshot,
    /// this will point to an alternative texture instead to allow for copying the render result
    /// to CPU memory.
    pub swap_chain_texture_view: Option<TextureView>,
    pub swap_chain_texture: Option<SurfaceTexture>,
    pub swap_chain_texture_format: Option<TextureFormat>,
    pub swap_chain_texture_view_format: Option<TextureFormat>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
    /// Whether this window needs an initial buffer commit.
    ///
    /// On Wayland, windows must present at least once before they are shown.
    /// See <https://wayland.app/protocols/xdg-shell#xdg_surface>
    pub needs_initial_present: bool,
    pub hdr_output: bool,
}

impl ExtractedWindow {
    fn set_swapchain_texture(&mut self, frame: wgpu::SurfaceTexture) {
        self.swap_chain_texture_view_format = Some(frame.texture.format().add_srgb_suffix());
        let texture_view_descriptor = TextureViewDescriptor {
            format: self.swap_chain_texture_view_format,
            ..default()
        };
        self.swap_chain_texture_view = Some(TextureView::from(
            frame.texture.create_view(&texture_view_descriptor),
        ));
        self.swap_chain_texture = Some(SurfaceTexture::from(frame));
    }

    fn has_swapchain_texture(&self) -> bool {
        self.swap_chain_texture_view.is_some() && self.swap_chain_texture.is_some()
    }

    pub fn present(&mut self) {
        if let Some(surface_texture) = self.swap_chain_texture.take() {
            // TODO(clean): winit docs recommends calling pre_present_notify before this.
            // though `present()` doesn't present the frame, it schedules it to be presented
            // by wgpu.
            // https://docs.rs/winit/0.29.9/wasm32-unknown-unknown/winit/window/struct.Window.html#method.pre_present_notify
            surface_texture.present();
        }
    }
}

#[derive(Default, Resource)]
pub struct ExtractedWindows {
    pub primary: Option<Entity>,
    pub windows: EntityHashMap<ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = EntityHashMap<ExtractedWindow>;

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
    mut closing: Extract<MessageReader<WindowClosing>>,
    windows: Extract<Query<(Entity, &Window, &RawHandleWrapper, Option<&PrimaryWindow>)>>,
    mut removed: Extract<RemovedComponents<RawHandleWrapper>>,
    mut window_surfaces: ResMut<WindowSurfaces>,
) {
    for (entity, window, handle, primary) in windows.iter() {
        if primary.is_some() {
            extracted_windows.primary = Some(entity);
        }

        let (new_width, new_height) = (
            window.resolution.physical_width().max(1),
            window.resolution.physical_height().max(1),
        );

        let extracted_window = extracted_windows.entry(entity).or_insert(ExtractedWindow {
            entity,
            handle: handle.clone(),
            physical_width: new_width,
            physical_height: new_height,
            present_mode: window.present_mode,
            desired_maximum_frame_latency: window.desired_maximum_frame_latency,
            swap_chain_texture: None,
            swap_chain_texture_view: None,
            size_changed: false,
            swap_chain_texture_format: None,
            swap_chain_texture_view_format: None,
            present_mode_changed: false,
            alpha_mode: window.composite_alpha_mode,
            needs_initial_present: true,
            hdr_output: window.hdr_output,
        });

        if extracted_window.swap_chain_texture.is_none() {
            // If we called present on the previous swap-chain texture last update,
            // then drop the swap chain frame here, otherwise we can keep it for the
            // next update as an optimization. `prepare_windows` will only acquire a new
            // swap chain texture if needed.
            extracted_window.swap_chain_texture_view = None;
        }
        extracted_window.size_changed = new_width != extracted_window.physical_width
            || new_height != extracted_window.physical_height;
        extracted_window.present_mode_changed =
            window.present_mode != extracted_window.present_mode;

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
                extracted_window.present_mode, window.present_mode
            );
            extracted_window.present_mode = window.present_mode;
        }

        extracted_window.hdr_output = window.hdr_output;
    }

    for closing_window in closing.read() {
        extracted_windows.remove(&closing_window.window);
        window_surfaces.remove(&closing_window.window);
    }
    for removed_window in removed.read() {
        extracted_windows.remove(&removed_window);
        window_surfaces.remove(&removed_window);
    }
}

struct SurfaceData {
    // TODO: what lifetime should this be?
    surface: WgpuWrapper<wgpu::Surface<'static>>,
    configuration: SurfaceConfiguration,
    texture_view_format: Option<TextureFormat>,
}

#[derive(Resource, Default)]
pub struct WindowSurfaces {
    surfaces: EntityHashMap<SurfaceData>,
    /// List of windows that we have already called the initial `configure_surface` for
    configured_windows: HashSet<Entity>,
}

impl WindowSurfaces {
    fn remove(&mut self, window: &Entity) {
        self.surfaces.remove(window);
        self.configured_windows.remove(window);
    }
}

/// (re)configures window surfaces, and obtains a swapchain texture for rendering.
///
/// NOTE: `get_current_texture` in `prepare_windows` can take a long time if the GPU workload is
/// the performance bottleneck. This can be seen in profiles as multiple prepare-set systems all
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
///   output during renderer initialization.
///   Another alternative is to try to use [`ANGLE`](https://github.com/gfx-rs/wgpu#angle) and
///   [`Backends::GL`](crate::settings::Backends::GL) with the `gles` feature enabled if your
///   GPU/drivers support `OpenGL 4.3` / `OpenGL ES 3.0` or later.
pub fn prepare_windows(
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_device: Res<RenderDevice>,
    #[cfg(target_os = "linux")] render_instance: Res<RenderInstance>,
) {
    for window in windows.windows.values_mut() {
        let window_surfaces = window_surfaces.deref_mut();
        let Some(surface_data) = window_surfaces.surfaces.get(&window.entity) else {
            continue;
        };

        // We didn't present the previous frame, so we can keep using our existing swapchain texture.
        if window.has_swapchain_texture()
            && !window.size_changed
            && !window.present_mode_changed
            && window.hdr_output
                == (surface_data.configuration.format == TextureFormat::Rgba16Float)
            && window.swap_chain_texture_format == Some(surface_data.configuration.format)
        {
            continue;
        }

        // A recurring issue is hitting `wgpu::SurfaceError::Timeout` on certain Linux
        // mesa driver implementations. This seems to be a quirk of some drivers.
        // We'd rather keep panicking when not on Linux mesa, because in those case,
        // the `Timeout` is still probably the symptom of a degraded unrecoverable
        // application state.
        // see https://github.com/bevyengine/bevy/pull/5957
        // and https://github.com/gfx-rs/wgpu/issues/1218
        #[cfg(target_os = "linux")]
        let may_erroneously_timeout = || {
            bevy_tasks::IoTaskPool::get().scope(|scope| {
                scope.spawn(async {
                    render_instance
                        .enumerate_adapters(wgpu::Backends::VULKAN)
                        .await
                        .iter()
                        .any(|adapter| {
                            let name = adapter.get_info().name;
                            name.starts_with("Radeon")
                                || name.starts_with("AMD")
                                || name.starts_with("Intel")
                        })
                });
            })[0]
        };

        let surface = &surface_data.surface;
        match surface.get_current_texture() {
            Ok(frame) => {
                window.set_swapchain_texture(frame);
            }
            Err(wgpu::SurfaceError::Outdated) => {
                render_device.configure_surface(surface, &surface_data.configuration);
                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(err) => {
                        // This is a common occurrence on X11 and Xwayland with NVIDIA drivers
                        // when opening and resizing the window.
                        warn!("Couldn't get swap chain texture after configuring. Cause: '{err}'");
                        continue;
                    }
                };
                window.set_swapchain_texture(frame);
            }
            #[cfg(target_os = "linux")]
            Err(wgpu::SurfaceError::Timeout) if may_erroneously_timeout() => {
                bevy_log::trace!(
                    "Couldn't get swap chain texture. This is probably a quirk \
                        of your Linux GPU driver, so it can be safely ignored."
                );
            }
            Err(err) => {
                panic!("Couldn't get swap chain texture, operation unrecoverable: {err}");
            }
        }
        window.swap_chain_texture_format = Some(surface_data.configuration.format);
    }
}

pub fn need_surface_configuration(
    windows: Res<ExtractedWindows>,
    window_surfaces: Res<WindowSurfaces>,
) -> bool {
    for window in windows.windows.values() {
        if !window_surfaces.configured_windows.contains(&window.entity)
            || window.size_changed
            || window.present_mode_changed
            || window.hdr_output
                != (window_surfaces
                    .surfaces
                    .get(&window.entity)
                    .is_some_and(|data| data.configuration.format == TextureFormat::Rgba16Float))
        {
            return true;
        }
    }
    false
}

// 2 is wgpu's default/what we've been using so far.
// 1 is the minimum, but may cause lower framerates due to the cpu waiting for the gpu to finish
// all work for the previous frame before starting work on the next frame, which then means the gpu
// has to wait for the cpu to finish to start on the next frame.
const DEFAULT_DESIRED_MAXIMUM_FRAME_LATENCY: u32 = 2;

/// Creates window surfaces.
pub fn create_surfaces(
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread,
    // which is necessary for some OS's
    #[cfg(any(target_os = "macos", target_os = "ios"))] _marker: bevy_ecs::system::NonSendMarker,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
) {
    for window in windows.windows.values_mut() {
        let data = window_surfaces
            .surfaces
            .entry(window.entity)
            .or_insert_with(|| {
                let surface_target = SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: window.handle.get_display_handle(),
                    raw_window_handle: window.handle.get_window_handle(),
                };
                // SAFETY: The window handles in ExtractedWindows will always be valid objects to create surfaces on
                let surface = unsafe {
                    // NOTE: On some OSes this MUST be called from the main thread.
                    // As of wgpu 0.15, only fallible if the given window is a HTML canvas and obtaining a WebGPU or WebGL2 context fails.
                    render_instance
                        .create_surface_unsafe(surface_target)
                        .expect("Failed to create wgpu surface")
                };
                let caps = surface.get_capabilities(&render_adapter);
                let present_mode = present_mode(window, &caps);
                let formats = caps.formats;
                // Prefer sRGB formats for surfaces, but fall back to first available format if no sRGB formats are available.
                let mut format = *formats.first().expect("No supported formats for surface");

                if window.hdr_output {
                    for available_format in &formats {
                        if *available_format == TextureFormat::Rgba16Float {
                            format = *available_format;
                            break;
                        }
                    }
                } else {
                    for available_format in formats {
                        // Rgba8UnormSrgb and Bgra8UnormSrgb and the only sRGB formats wgpu exposes that we can use for surfaces.
                        if available_format == TextureFormat::Rgba8UnormSrgb
                            || available_format == TextureFormat::Bgra8UnormSrgb
                        {
                            format = available_format;
                            break;
                        }
                    }
                }

                let texture_view_format = if !format.is_srgb() {
                    Some(format.add_srgb_suffix())
                } else {
                    None
                };
                let configuration = SurfaceConfiguration {
                    format,
                    width: window.physical_width,
                    height: window.physical_height,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    present_mode,
                    desired_maximum_frame_latency: window
                        .desired_maximum_frame_latency
                        .map(NonZero::<u32>::get)
                        .unwrap_or(DEFAULT_DESIRED_MAXIMUM_FRAME_LATENCY),
                    alpha_mode: match window.alpha_mode {
                        CompositeAlphaMode::Auto => wgpu::CompositeAlphaMode::Auto,
                        CompositeAlphaMode::Opaque => wgpu::CompositeAlphaMode::Opaque,
                        CompositeAlphaMode::PreMultiplied => {
                            wgpu::CompositeAlphaMode::PreMultiplied
                        }
                        CompositeAlphaMode::PostMultiplied => {
                            wgpu::CompositeAlphaMode::PostMultiplied
                        }
                        CompositeAlphaMode::Inherit => wgpu::CompositeAlphaMode::Inherit,
                    },
                    view_formats: match texture_view_format {
                        Some(format) => vec![format],
                        None => vec![],
                    },
                };

                render_device.configure_surface(&surface, &configuration);

                SurfaceData {
                    surface: WgpuWrapper::new(surface),
                    configuration,
                    texture_view_format,
                }
            });

        if window.size_changed
            || window.present_mode_changed
            || window.hdr_output != (data.configuration.format == TextureFormat::Rgba16Float)
        {
            // normally this is dropped on present but we double check here to be safe as failure to
            // drop it will cause validation errors in wgpu
            drop(window.swap_chain_texture.take());
            #[cfg_attr(
                target_arch = "wasm32",
                expect(clippy::drop_non_drop, reason = "texture views are not drop on wasm")
            )]
            drop(window.swap_chain_texture_view.take());

            data.configuration.width = window.physical_width;
            data.configuration.height = window.physical_height;
            let caps = data.surface.get_capabilities(&render_adapter);
            data.configuration.present_mode = present_mode(window, &caps);

            let formats = caps.formats;
            let mut format = *formats.first().expect("No supported formats for surface");

            if window.hdr_output {
                for available_format in &formats {
                    if *available_format == TextureFormat::Rgba16Float {
                        format = *available_format;
                        break;
                    }
                }
            } else {
                for available_format in formats {
                    // Rgba8UnormSrgb and Bgra8UnormSrgb and the only sRGB formats wgpu exposes that we can use for surfaces.
                    if available_format == TextureFormat::Rgba8UnormSrgb
                        || available_format == TextureFormat::Bgra8UnormSrgb
                    {
                        format = available_format;
                        break;
                    }
                }
            }

            if data.configuration.format != format {
                data.configuration.format = format;
                data.texture_view_format = if !format.is_srgb() {
                    Some(format.add_srgb_suffix())
                } else {
                    None
                };
                data.configuration.view_formats = match data.texture_view_format {
                    Some(format) => vec![format],
                    None => vec![],
                };
            }

            render_device.configure_surface(&data.surface, &data.configuration);
        }

        window_surfaces.configured_windows.insert(window.entity);
    }
}

fn present_mode(
    window: &mut ExtractedWindow,
    caps: &wgpu::SurfaceCapabilities,
) -> wgpu::PresentMode {
    let present_mode = match window.present_mode {
        PresentMode::Fifo => wgpu::PresentMode::Fifo,
        PresentMode::FifoRelaxed => wgpu::PresentMode::FifoRelaxed,
        PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
        PresentMode::Immediate => wgpu::PresentMode::Immediate,
        PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
        PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
    };
    let fallbacks = match present_mode {
        wgpu::PresentMode::AutoVsync => {
            &[wgpu::PresentMode::FifoRelaxed, wgpu::PresentMode::Fifo][..]
        }
        wgpu::PresentMode::AutoNoVsync => &[
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Fifo,
        ][..],
        wgpu::PresentMode::Mailbox => &[
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Fifo,
        ][..],
        // Always end in FIFO to make sure it's always supported
        x => &[x, wgpu::PresentMode::Fifo][..],
    };
    let new_present_mode = fallbacks
        .iter()
        .copied()
        .find(|fallback| caps.present_modes.contains(fallback))
        .unwrap_or_else(|| {
            unreachable!(
                "Fallback system failed to choose present mode. \
                            This is a bug. Mode: {:?}, Options: {:?}",
                window.present_mode, &caps.present_modes
            );
        });
    if new_present_mode != present_mode && fallbacks.contains(&present_mode) {
        info!("PresentMode {present_mode:?} requested but not available. Falling back to {new_present_mode:?}");
    }
    new_present_mode
}
