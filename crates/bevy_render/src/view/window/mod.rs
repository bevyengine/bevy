use crate::renderer::WgpuWrapper;
use crate::sync_world::{MainEntity, RenderEntity, SyncToRenderWorld};
use crate::{camera::extract_cameras, renderer::RenderQueue};
use crate::{
    render_resource::{SurfaceTexture, TextureView},
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use bevy_log::{debug, info, warn};
use bevy_utils::default;
use bevy_window::{
    CompositeAlphaMode, PresentMode, PrimaryWindow, RawHandleWrapper, Window, WindowClosing,
};
use core::num::NonZero;
use wgpu::{
    SurfaceConfiguration, SurfaceTargetUnsafe, TextureFormat, TextureUsages, TextureViewDescriptor,
};

pub mod screenshot;

use screenshot::ScreenshotPlugin;

pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScreenshotPlugin);

        // We need to sync the window entity in the render world
        // We can't use [`SyncComponentPlugin`] because it would introduce `bevy_render` as
        // a dependency to `bevy_window`
        {
            app.add_observer(|trigger: On<Add, Window>, mut commands: Commands| {
                commands.entity(trigger.entity).insert(SyncToRenderWorld);
            });

            // The primary window gets added before this plugin so we can't rely on the observer
            let _ = app.world_mut().run_system_once(
                |mut commands: Commands, windows: Query<Entity, With<Window>>| {
                    for entity in &windows {
                        commands.entity(entity).insert(SyncToRenderWorld);
                    }
                },
            );
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(ExtractSchedule, extract_windows.before(extract_cameras))
                .add_systems(
                    Render,
                    create_surfaces
                        .run_if(need_surface_configuration)
                        .before(prepare_windows),
                )
                .add_systems(Render, prepare_windows.in_set(RenderSystems::PrepareViews));
        }
    }
}

#[derive(Component)]
pub struct ExtractedWindow {
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
    /// This is an srgb view of [`ExtractedWindow::swap_chain_texture_format`]
    /// so that in shaders we are always in linear space.
    pub swap_chain_texture_view_format: Option<TextureFormat>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
    /// Whether this window needs an initial buffer commit.
    ///
    /// On Wayland, windows must present at least once before they are shown.
    /// See <https://wayland.app/protocols/xdg-shell#xdg_surface>
    pub needs_initial_present: bool,
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

    pub fn present(&mut self, queue: &RenderQueue) {
        if let Some(surface_texture) = self.swap_chain_texture.take() {
            // TODO(clean): winit docs recommends calling pre_present_notify before this.
            // though `present()` doesn't present the frame, it schedules it to be presented
            // by wgpu.
            // https://docs.rs/winit/0.29.9/wasm32-unknown-unknown/winit/window/struct.Window.html#method.pre_present_notify
            surface_texture.present(queue);
        }
    }
}

fn extract_windows(
    mut commands: Commands,
    mut extracted_windows: Query<&mut ExtractedWindow>,
    mut closing: Extract<MessageReader<WindowClosing>>,
    windows: Extract<Query<(RenderEntity, &Window, &RawHandleWrapper, Has<PrimaryWindow>)>>,
    mut removed: Extract<RemovedComponents<RawHandleWrapper>>,
    mut removed_primary: Extract<RemovedComponents<PrimaryWindow>>,
    mapper: Extract<Query<&RenderEntity>>,
) {
    for (render_entity, window, handle, is_primary) in windows.iter() {
        if is_primary {
            commands.entity(render_entity).insert(PrimaryWindow);
        }

        let (new_width, new_height) = (
            window.resolution.physical_width().max(1),
            window.resolution.physical_height().max(1),
        );

        let Ok(mut extracted_window) = extracted_windows.get_mut(render_entity) else {
            commands.entity(render_entity).insert((
                ExtractedWindow {
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
                },
                handle.clone(),
            ));
            continue;
        };

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
        if let Ok(render_entity) = mapper.get(closing_window.window) {
            commands.entity(render_entity.entity()).despawn();
        }
    }
    for removed_window in removed.read() {
        if let Ok(render_entity) = mapper.get(removed_window) {
            commands.entity(render_entity.entity()).despawn();
        }
    }
    for removed_window in removed_primary.read() {
        if let Ok(render_entity) = mapper.get(removed_window) {
            commands
                .entity(render_entity.entity())
                .remove::<PrimaryWindow>();
        }
    }
}

#[derive(Component)]
pub struct SurfaceData {
    // TODO: what lifetime should this be?
    surface: WgpuWrapper<wgpu::Surface<'static>>,
    configuration: SurfaceConfiguration,
    texture_view_format: Option<TextureFormat>,
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
    mut windows: Query<(MainEntity, &mut ExtractedWindow, Option<&SurfaceData>)>,
    render_device: Res<RenderDevice>,
    sorted_cameras: Res<crate::camera::SortedCameras>,
    #[cfg(target_os = "linux")] render_instance: Res<RenderInstance>,
) {
    for (main_entity, mut window, maybe_surface_data) in &mut windows {
        // Skip acquiring a swap-chain texture for windows that no camera
        // targets. This avoids a wasted clear pass in
        // `handle_uncovered_swap_chains` that triggers a DMA-fence fd leak on
        // Adreno 740 (Quest 3). The exception is windows that still need their
        // initial present (required on Wayland).
        let is_camera_target = sorted_cameras.0.iter().any(|c| {
            matches!(
                &c.target,
                Some(bevy_camera::NormalizedRenderTarget::Window(w)) if w.entity() == main_entity
            ) && matches!(c.output_mode, bevy_camera::CameraOutputMode::Write { .. })
        });
        if !is_camera_target && !window.needs_initial_present {
            continue;
        }

        let Some(surface_data) = maybe_surface_data else {
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
            wgpu::CurrentSurfaceTexture::Success(surface_texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                window.set_swapchain_texture(surface_texture);
            }
            #[cfg(target_os = "linux")]
            wgpu::CurrentSurfaceTexture::Timeout if may_erroneously_timeout() => {
                bevy_log::trace!(
                    "Couldn't get swap chain texture. This is probably a quirk \
                        of your Linux GPU driver, so it can be safely ignored."
                );
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                render_device.configure_surface(surface, &surface_data.configuration);
                let frame = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(surface_texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
                    variant => {
                        // This is a common occurrence on X11 and Xwayland with NVIDIA drivers
                        // when opening and resizing the window.
                        warn!(
                            "Couldn't get swap chain texture after configuring. Cause: '{variant:?}'"
                        );
                        continue;
                    }
                };
                window.set_swapchain_texture(frame);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {}
            other => {
                bevy_log::error!("Couldn't get swap chain texture: {other:?}");
            }
        }
        window.swap_chain_texture_format = Some(surface_data.configuration.format);
    }
}

pub fn need_surface_configuration(windows: Query<(&ExtractedWindow, Has<SurfaceData>)>) -> bool {
    for (window, has_surface_data) in &windows {
        if !has_surface_data || window.size_changed || window.present_mode_changed {
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
    mut commands: Commands,
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread,
    // which is necessary for some OS's
    #[cfg(any(target_os = "macos", target_os = "ios"))] _marker: bevy_ecs::system::NonSendMarker,
    mut windows: Query<(
        Entity,
        &mut ExtractedWindow,
        &RawHandleWrapper,
        Option<&mut SurfaceData>,
    )>,
    render_instance: Res<RenderInstance>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
) {
    for (entity, mut window, handle, mut maybe_surface_data) in &mut windows {
        let Some(data) = maybe_surface_data.as_mut() else {
            let surface_target = SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(handle.get_display_handle()),
                raw_window_handle: handle.get_window_handle(),
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
            let present_mode = present_mode(&window, &caps);
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
                present_mode,
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
                color_space: wgpu::SurfaceColorSpace::Auto,
            };

            render_device.configure_surface(&surface, &configuration);

            commands.entity(entity).insert(SurfaceData {
                surface: WgpuWrapper::new(surface),
                configuration,
                texture_view_format,
            });
            continue;
        };

        if window.size_changed || window.present_mode_changed {
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
            data.configuration.present_mode = present_mode(&window, &caps);
            render_device.configure_surface(&data.surface, &data.configuration);
        }
    }
}

fn present_mode(window: &ExtractedWindow, caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
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
