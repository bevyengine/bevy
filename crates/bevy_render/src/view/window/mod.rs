use crate::{
    render_resource::{
        BindGroupEntries, PipelineCache, SpecializedRenderPipelines, SurfaceTexture, TextureView,
    },
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    texture::TextureFormatPixelInfo,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet, WgpuWrapper,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{entity::EntityHashMap, prelude::*};
#[cfg(target_os = "linux")]
use bevy_utils::warn_once;
use bevy_utils::{default, tracing::debug, HashSet};
use bevy_window::{
    CompositeAlphaMode, PresentMode, PrimaryWindow, RawHandleWrapper, Window, WindowClosing,
};
use std::{
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    sync::PoisonError,
};
use wgpu::{
    BufferUsages, SurfaceConfiguration, SurfaceTargetUnsafe, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

pub mod screenshot;

use screenshot::{
    ScreenshotManager, ScreenshotPlugin, ScreenshotPreparedState, ScreenshotToScreenPipeline,
};

use super::Msaa;

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
                .add_systems(Render, prepare_windows.in_set(RenderSet::ManageViews));
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<ScreenshotToScreenPipeline>();
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
    pub desired_maximum_frame_latency: Option<NonZeroU32>,
    /// Note: this will not always be the swap chain texture view. When taking a screenshot,
    /// this will point to an alternative texture instead to allow for copying the render result
    /// to CPU memory.
    pub swap_chain_texture_view: Option<TextureView>,
    pub swap_chain_texture: Option<SurfaceTexture>,
    pub swap_chain_texture_format: Option<TextureFormat>,
    pub screenshot_memory: Option<ScreenshotPreparedState>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
    pub screenshot_func: Option<screenshot::ScreenshotFn>,
}

impl ExtractedWindow {
    fn set_swapchain_texture(&mut self, frame: wgpu::SurfaceTexture) {
        let texture_view_descriptor = TextureViewDescriptor {
            format: Some(frame.texture.format().add_srgb_suffix()),
            ..default()
        };
        self.swap_chain_texture_view = Some(TextureView::from(
            frame.texture.create_view(&texture_view_descriptor),
        ));
        self.swap_chain_texture = Some(SurfaceTexture::from(frame));
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
    screenshot_manager: Extract<Res<ScreenshotManager>>,
    mut closing: Extract<EventReader<WindowClosing>>,
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
            present_mode_changed: false,
            alpha_mode: window.composite_alpha_mode,
            screenshot_func: None,
            screenshot_memory: None,
        });

        // NOTE: Drop the swap chain frame here
        extracted_window.swap_chain_texture_view = None;
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
    for removed_window in removed.read() {
        extracted_windows.remove(&removed_window);
        window_surfaces.remove(&removed_window);
    }
    // This lock will never block because `callbacks` is `pub(crate)` and this is the singular callsite where it's locked.
    // Even if a user had multiple copies of this system, since the system has a mutable resource access the two systems would never run
    // at the same time
    // TODO: since this is guaranteed, should the lock be replaced with an UnsafeCell to remove the overhead, or is it minor enough to be ignored?
    for (window, screenshot_func) in screenshot_manager
        .callbacks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .drain()
    {
        if let Some(window) = extracted_windows.get_mut(&window) {
            window.screenshot_func = Some(screenshot_func);
        }
    }
}

struct SurfaceData {
    // TODO: what lifetime should this be?
    surface: WgpuWrapper<wgpu::Surface<'static>>,
    configuration: SurfaceConfiguration,
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

#[cfg(target_os = "linux")]
const NVIDIA_VENDOR_ID: u32 = 0x10DE;

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
///   output during renderer initialization. Future versions of wgpu will support `DirectX 11`, but
///   another alternative is to try to use [`ANGLE`](https://github.com/gfx-rs/wgpu#angle) and
///   [`Backends::GL`](crate::settings::Backends::GL) if your GPU/drivers support `OpenGL 4.3` / `OpenGL ES 3.0` or
///   later.
#[allow(clippy::too_many_arguments)]
pub fn prepare_windows(
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    screenshot_pipeline: Res<ScreenshotToScreenPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ScreenshotToScreenPipeline>>,
    mut msaa: ResMut<Msaa>,
    #[cfg(target_os = "linux")] render_instance: Res<RenderInstance>,
) {
    for window in windows.windows.values_mut() {
        let window_surfaces = window_surfaces.deref_mut();
        let Some(surface_data) = window_surfaces.surfaces.get(&window.entity) else {
            continue;
        };

        // This is an ugly hack to work around drivers that don't support MSAA.
        // This should be removed once https://github.com/bevyengine/bevy/issues/7194 lands and we're doing proper
        // feature detection for MSAA.
        // When removed, we can also remove the `.after(prepare_windows)` of `prepare_core_3d_depth_textures` and `prepare_prepass_textures`
        let sample_flags = render_adapter
            .get_texture_format_features(surface_data.configuration.format)
            .flags;

        if !sample_flags.sample_count_supported(msaa.samples()) {
            let fallback = if sample_flags.sample_count_supported(Msaa::default().samples()) {
                Msaa::default()
            } else {
                Msaa::Off
            };

            let fallback_str = if fallback == Msaa::Off {
                "disabling MSAA".to_owned()
            } else {
                format!("MSAA {}x", fallback.samples())
            };

            bevy_utils::tracing::warn!(
                "MSAA {}x is not supported on this device. Falling back to {}.",
                msaa.samples(),
                fallback_str,
            );
            *msaa = fallback;
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

        #[cfg(target_os = "linux")]
        let is_nvidia = || {
            render_instance
                .enumerate_adapters(wgpu::Backends::VULKAN)
                .iter()
                .any(|adapter| adapter.get_info().vendor & 0xFFFF == NVIDIA_VENDOR_ID)
        };

        let not_already_configured = window_surfaces.configured_windows.insert(window.entity);

        let surface = &surface_data.surface;
        if not_already_configured || window.size_changed || window.present_mode_changed {
            match surface.get_current_texture() {
                Ok(frame) => window.set_swapchain_texture(frame),
                #[cfg(target_os = "linux")]
                Err(wgpu::SurfaceError::Outdated) if is_nvidia() => {
                    warn_once!(
                        "Couldn't get swap chain texture. This often happens with \
                        the NVIDIA drivers on Linux. It can be safely ignored."
                    );
                }
                Err(err) => panic!("Error configuring surface: {err}"),
            };
        } else {
            match surface.get_current_texture() {
                Ok(frame) => {
                    window.set_swapchain_texture(frame);
                }
                #[cfg(target_os = "linux")]
                Err(wgpu::SurfaceError::Outdated) if is_nvidia() => {
                    warn_once!(
                        "Couldn't get swap chain texture. This often happens with \
                        the NVIDIA drivers on Linux. It can be safely ignored."
                    );
                }
                Err(wgpu::SurfaceError::Outdated) => {
                    render_device.configure_surface(surface, &surface_data.configuration);
                    let frame = surface
                        .get_current_texture()
                        .expect("Error reconfiguring surface");
                    window.set_swapchain_texture(frame);
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
        window.swap_chain_texture_format = Some(surface_data.configuration.format);

        if window.screenshot_func.is_some() {
            let texture = render_device.create_texture(&wgpu::TextureDescriptor {
                label: Some("screenshot-capture-rendertarget"),
                size: wgpu::Extent3d {
                    width: surface_data.configuration.width,
                    height: surface_data.configuration.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface_data.configuration.format.add_srgb_suffix(),
                usage: TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::COPY_SRC
                    | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let texture_view = texture.create_view(&Default::default());
            let buffer = render_device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screenshot-transfer-buffer"),
                size: screenshot::get_aligned_size(
                    window.physical_width,
                    window.physical_height,
                    surface_data.configuration.format.pixel_size() as u32,
                ) as u64,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = render_device.create_bind_group(
                "screenshot-to-screen-bind-group",
                &screenshot_pipeline.bind_group_layout,
                &BindGroupEntries::single(&texture_view),
            );
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &screenshot_pipeline,
                surface_data.configuration.format,
            );
            window.swap_chain_texture_view = Some(texture_view);
            window.screenshot_memory = Some(ScreenshotPreparedState {
                texture,
                buffer,
                bind_group,
                pipeline_id,
            });
        }
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
    #[cfg(any(target_os = "macos", target_os = "ios"))] _marker: Option<
        NonSend<bevy_core::NonSendMarker>,
    >,
    windows: Res<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
) {
    for window in windows.windows.values() {
        let data = window_surfaces
            .surfaces
            .entry(window.entity)
            .or_insert_with(|| {
                let surface_target = SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: window.handle.display_handle,
                    raw_window_handle: window.handle.window_handle,
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

                let configuration = wgpu::SurfaceConfiguration {
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
                        .map(NonZeroU32::get)
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
                    view_formats: if !format.is_srgb() {
                        vec![format.add_srgb_suffix()]
                    } else {
                        vec![]
                    },
                };

                render_device.configure_surface(&surface, &configuration);

                SurfaceData {
                    surface: WgpuWrapper::new(surface),
                    configuration,
                }
            });

        if window.size_changed || window.present_mode_changed {
            data.configuration.width = window.physical_width;
            data.configuration.height = window.physical_height;
            data.configuration.present_mode = match window.present_mode {
                PresentMode::Fifo => wgpu::PresentMode::Fifo,
                PresentMode::FifoRelaxed => wgpu::PresentMode::FifoRelaxed,
                PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
                PresentMode::Immediate => wgpu::PresentMode::Immediate,
                PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
                PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
            };
            render_device.configure_surface(&data.surface, &data.configuration);
        }
    }
}
