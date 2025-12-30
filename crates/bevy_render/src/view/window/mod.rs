use crate::{
    render_resource::{SurfaceTexture, TextureView},
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_platform::collections::{hash_map::Entry, HashSet};
use bevy_utils::default;
use bevy_window::{CompositeAlphaMode, PresentMode, PrimaryWindow, Window, WindowClosing};
use core::{
    num::NonZero,
    ops::{Deref, DerefMut},
};
use surface_target::{RenderSurface, SurfaceCreationError, SurfaceTargetSource};
use tracing::{debug, error, info, warn};
use wgpu::{SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor};

pub mod screenshot;
pub mod surface_target;

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
                    (create_send_surfaces, create_non_send_surfaces)
                        .chain()
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
    pub surface_target_source: SurfaceTargetSource,
    /// Set to `true` if surface creation failed. Repeated attempts will not be tried.
    pub surface_failed: bool,
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
    windows: Extract<
        Query<(
            Entity,
            &Window,
            &SurfaceTargetSource,
            Option<&PrimaryWindow>,
        )>,
    >,
    mut removed: Extract<RemovedComponents<SurfaceTargetSource>>,
    mut window_surfaces: ResMut<WindowSurfaces>,
) {
    for removed_window in removed.read() {
        extracted_windows.remove(&removed_window);
        window_surfaces.remove(&removed_window);
    }

    for (entity, window, surface_target_source, primary) in windows.iter() {
        if primary.is_some() {
            extracted_windows.primary = Some(entity);
        }

        let (new_width, new_height) = (
            window.resolution.physical_width().max(1),
            window.resolution.physical_height().max(1),
        );

        let extracted_window = extracted_windows.entry(entity).or_insert(ExtractedWindow {
            entity,
            surface_failed: false,
            surface_target_source: surface_target_source.clone(),
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
    }

    for closing_window in closing.read() {
        extracted_windows.remove(&closing_window.window);
        window_surfaces.remove(&closing_window.window);
    }
}

struct SurfaceData {
    surface: RenderSurface,
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
        if window.has_swapchain_texture() && !window.size_changed && !window.present_mode_changed {
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
            render_instance
                .enumerate_adapters(wgpu::Backends::VULKAN)
                .iter()
                .any(|adapter| {
                    let name = adapter.get_info().name;
                    name.starts_with("Radeon")
                        || name.starts_with("AMD")
                        || name.starts_with("Intel")
                })
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
                tracing::trace!(
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
        if (!window_surfaces.configured_windows.contains(&window.entity)
            || window.size_changed
            || window.present_mode_changed)
            && !window.surface_failed
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

fn create_window_surface(
    is_main_thread: bool,
    window: &ExtractedWindow,
    render_instance: &RenderInstance,
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) -> Result<SurfaceData, SurfaceCreationError> {
    let surface = window
        .surface_target_source
        .create_surface(render_instance, is_main_thread)?;

    let caps = surface.get_capabilities(render_adapter);
    let formats = caps.formats;
    // For future HDR output support, we'll need to request a format that supports HDR,
    // but as of wgpu 0.15 that is not yet supported.
    // Prefer sRGB formats for surfaces, but fall back to first available format if no sRGB formats are available.
    let mut format = *formats.first().expect("No supported formats for surface");
    for available_format in formats {
        // Rgba8UnormSrgb and Bgra8UnormSrgb and the only sRGB formats wgpu exposes that we can use for surfaces.
        if available_format == TextureFormat::Rgba8UnormSrgb
            || available_format == TextureFormat::Bgra8UnormSrgb
        {
            format = available_format;
            break;
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
        present_mode: match window.present_mode {
            PresentMode::Fifo => wgpu::PresentMode::Fifo,
            PresentMode::FifoRelaxed => wgpu::PresentMode::FifoRelaxed,
            PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
            PresentMode::Immediate => wgpu::PresentMode::Immediate,
            PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
            PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
        },
        desired_maximum_frame_latency: window
            .desired_maximum_frame_latency
            .map(NonZero::<u32>::get)
            .unwrap_or(DEFAULT_DESIRED_MAXIMUM_FRAME_LATENCY),
        alpha_mode: match window.alpha_mode {
            CompositeAlphaMode::Auto => wgpu::CompositeAlphaMode::Auto,
            CompositeAlphaMode::Opaque => wgpu::CompositeAlphaMode::Opaque,
            CompositeAlphaMode::PreMultiplied => wgpu::CompositeAlphaMode::PreMultiplied,
            CompositeAlphaMode::PostMultiplied => wgpu::CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::Inherit => wgpu::CompositeAlphaMode::Inherit,
        },
        view_formats: match texture_view_format {
            Some(format) => vec![format],
            None => vec![],
        },
    };

    render_device.configure_surface(&surface, &configuration);

    Ok(SurfaceData {
        surface,
        configuration,
        texture_view_format,
    })
}

fn reconfigure_window_surface(
    window: &mut ExtractedWindow,
    data: &mut SurfaceData,
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) {
    // normally this is dropped on present but we double check here to be safe as failure to
    // drop it will cause validation errors in wgpu
    drop(window.swap_chain_texture.take());
    drop(window.swap_chain_texture_view.take());

    data.configuration.width = window.physical_width;
    data.configuration.height = window.physical_height;
    let caps = data.surface.get_capabilities(&render_adapter);
    data.configuration.present_mode = present_mode(window, &caps);
    render_device.configure_surface(&data.surface, &data.configuration);
}

/// Creates window surfaces that do not require the main thread.
pub fn create_send_surfaces(
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
) {
    let render_instance = render_instance.as_ref();
    let render_device = render_device.as_ref();
    let render_adapter = render_adapter.as_ref();

    let is_main_thread = false;
    let send_windows = windows
        .windows
        .values_mut()
        .filter(|window| !window.surface_target_source.requires_main_thread());

    for window in send_windows {
        match window_surfaces.surfaces.entry(window.entity) {
            Entry::Vacant(entry) => {
                match create_window_surface(
                    is_main_thread,
                    window,
                    render_instance,
                    render_device,
                    render_adapter,
                ) {
                    Ok(data) => {
                        entry.insert(data);
                        window_surfaces.configured_windows.insert(window.entity);
                    }
                    Err(err) => {
                        window.surface_failed = true;
                        error!(
                            "Window {:?} surface creation failed: {:?}",
                            window.entity, err
                        );
                        continue;
                    }
                }
            }
            Entry::Occupied(mut entry) => {
                if window.size_changed || window.present_mode_changed {
                    let data = entry.get_mut();
                    reconfigure_window_surface(window, data, render_device, render_adapter);
                }
            }
        };
    }
}

/// Creates window surfaces that require the main thread.
pub fn create_non_send_surfaces(
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread
    _marker: bevy_ecs::system::NonSendMarker,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
) {
    let render_instance = render_instance.as_ref();
    let render_device = render_device.as_ref();
    let render_adapter = render_adapter.as_ref();

    let is_main_thread: bool = true;
    let non_send_windows = windows
        .windows
        .values_mut()
        .filter(|window| window.surface_target_source.requires_main_thread());

    for window in non_send_windows {
        match window_surfaces.surfaces.entry(window.entity) {
            Entry::Vacant(entry) => {
                match create_window_surface(
                    is_main_thread,
                    window,
                    render_instance,
                    render_device,
                    render_adapter,
                ) {
                    Ok(data) => {
                        entry.insert(data);
                        window_surfaces.configured_windows.insert(window.entity);
                    }
                    Err(err) => {
                        window.surface_failed = true;
                        error!(
                            "Window {:?} surface creation failed: {:?}",
                            window.entity, err
                        );
                        continue;
                    }
                }
            }
            Entry::Occupied(mut entry) => {
                if window.size_changed || window.present_mode_changed {
                    let data = entry.get_mut();
                    reconfigure_window_surface(window, data, render_device, render_adapter);
                }
            }
        };
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
