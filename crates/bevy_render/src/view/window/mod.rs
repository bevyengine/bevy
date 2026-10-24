use crate::renderer::wgpu_wrapper;
use crate::sync_world::{MainEntity, RenderEntity, SyncToRenderWorld};
use crate::{camera::extract_cameras, renderer::RenderQueue};
use crate::{
    render_resource::{SurfaceTexture, TextureView},
    renderer::{RenderAdapter, RenderDevice, RenderInstance},
    Extract, ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use bevy_log::{debug, info, info_once, warn, warn_once};
use bevy_utils::default;
use bevy_window::{
    CompositeAlphaMode, DisplayTarget, PresentMode, PrimaryWindow, RawHandleWrapper,
    SurfaceColorSpace, SurfaceColorSpaces, Window, WindowClosing, WindowSurfaceColorSpaces,
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
            app.add_observer(|trigger: On<Add<Window>>, mut commands: Commands| {
                commands
                    .entity(trigger.entity)
                    .insert(SyncToRenderWorld::default());
            });

            // The primary window gets added before this plugin so we can't rely on the observer
            let _ = app.world_mut().run_system_once(
                |mut commands: Commands, windows: Query<Entity, With<Window>>| {
                    for entity in &windows {
                        commands.entity(entity).insert(SyncToRenderWorld::default());
                    }
                },
            );
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_windows.before(extract_cameras),
                        write_back_surface_color_spaces.after(extract_windows),
                    ),
                )
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
    /// Under an HDR color space it equals the surface format, with no sRGB
    /// view.
    pub swap_chain_texture_view_format: Option<TextureFormat>,
    pub size_changed: bool,
    pub present_mode_changed: bool,
    pub alpha_mode: CompositeAlphaMode,
    /// The window's requested [`DisplayTarget`].
    pub display_target: DisplayTarget,
    /// Whether [`DisplayTarget::hdr`] or [`DisplayTarget::color_space_override`]
    /// changed, so the surface must be renegotiated. A luminance change does
    /// not reconfigure the surface.
    pub color_space_request_changed: bool,
    /// The [`SurfaceColorSpace`] the configured surface uses. `None` until
    /// [`create_surfaces`] has configured the surface.
    pub resolved_color_space: Option<SurfaceColorSpace>,
    /// Whether this window needs an initial buffer commit.
    ///
    /// On Wayland, windows must present at least once before they are shown.
    /// See <https://wayland.app/protocols/xdg-shell#xdg_surface>
    pub needs_initial_present: bool,
}

impl ExtractedWindow {
    fn set_swapchain_texture(
        &mut self,
        frame: wgpu::SurfaceTexture,
        texture_view_format: Option<TextureFormat>,
    ) {
        // `add_srgb_suffix` would give an sRGB view to a non-sRGB format an HDR
        // negotiation chose.
        self.swap_chain_texture_view_format =
            Some(texture_view_format.unwrap_or_else(|| frame.texture.format()));
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
    windows: Extract<
        Query<(
            RenderEntity,
            &Window,
            Option<&DisplayTarget>,
            &RawHandleWrapper,
            Has<PrimaryWindow>,
        )>,
    >,
    mut removed: Extract<RemovedComponents<RawHandleWrapper>>,
    mut removed_primary: Extract<RemovedComponents<PrimaryWindow>>,
    mapper: Extract<Query<&RenderEntity>>,
) {
    for (render_entity, window, display_target, handle, is_primary) in windows.iter() {
        // A required component can still be removed. Fall back to the default
        // rather than drop the window.
        let display_target = display_target.copied().unwrap_or_default();
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
                    display_target,
                    color_space_request_changed: false,
                    resolved_color_space: None,
                    needs_initial_present: true,
                },
                handle.clone(),
            ));
            continue;
        };

        extracted_window.color_space_request_changed =
            color_space_request_changed(&extracted_window.display_target, &display_target);
        extracted_window.display_target = display_target;

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

    // Remove the components instead of despawn the synced render entity here:
    // `RawHandleWrapper` removal is not necessarily
    // terminal. On Android the native window is destroyed (and its handle removed) when the app
    // suspends, but the window entity survives and is given a new handle on resume. Despawning
    // would leave the `SubEntity` on the main world entity pointing at a dead entity, and the
    // next round of extraction would panic when reusing it.
    //
    // The render entity is only ever despawned by the entity sync system once the main world
    // window entity is actually destroyed. Here we only tear down the surface and drop the
    // extracted window data; extraction and `create_surfaces` will recreate them whenever the
    // window has a (new) `RawHandleWrapper` again.
    for closing_window in closing.read() {
        if let Ok(render_entity) = mapper.get(closing_window.window) {
            commands
                .entity(render_entity.entity())
                .remove::<(ExtractedWindow, RawHandleWrapper, SurfaceData)>();
        }
    }
    for removed_window in removed.read() {
        if let Ok(render_entity) = mapper.get(removed_window) {
            commands
                .entity(render_entity.entity())
                .remove::<(ExtractedWindow, RawHandleWrapper, SurfaceData)>();
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

/// Returns `true` if the change from `previous` to `current` needs a surface
/// renegotiation: [`DisplayTarget::hdr`] or
/// [`DisplayTarget::color_space_override`] differ. A luminance change does
/// not.
fn color_space_request_changed(previous: &DisplayTarget, current: &DisplayTarget) -> bool {
    previous.hdr != current.hdr || previous.color_space_override != current.color_space_override
}

/// Inserts `value` only if it differs from the current component, so change
/// detection fires only on real changes. Does nothing if `entity` is gone.
fn insert_on_change<C: Component + PartialEq>(
    main_world: &mut MainWorld,
    entity: Entity,
    value: C,
) {
    let Ok(mut entity_mut) = main_world.get_entity_mut(entity) else {
        return;
    };
    if entity_mut.get::<C>() != Some(&value) {
        entity_mut.insert(value);
    }
}

/// Writes each window's [`WindowSurfaceColorSpaces`] back to the main world.
///
/// It runs during extraction, so the main world sees the previous frame's
/// result.
fn write_back_surface_color_spaces(
    mut main_world: ResMut<MainWorld>,
    windows: Query<(MainEntity, &SurfaceData)>,
) {
    for (entity, surface_data) in windows.iter() {
        insert_on_change(
            &mut main_world,
            entity,
            WindowSurfaceColorSpaces {
                resolved: surface_data.resolved_color_space,
                supported: surface_data.supported_color_spaces,
            },
        );
    }
}

// TODO: what lifetime should this be?
wgpu_wrapper!(struct WgpuSurface(wgpu::Surface<'static>));

#[derive(Component)]
pub struct SurfaceData {
    surface: WgpuSurface,
    configuration: SurfaceConfiguration,
    texture_view_format: Option<TextureFormat>,
    /// The [`SurfaceColorSpace`] the surface is configured for.
    resolved_color_space: SurfaceColorSpace,
    /// The color spaces the surface can provide, refreshed by
    /// [`SurfaceData::renegotiate`] whenever the surface is reconfigured.
    supported_color_spaces: SurfaceColorSpaces,
}

impl SurfaceData {
    fn apply_negotiated(&mut self, negotiated: NegotiatedSurface) {
        self.configuration.format = negotiated.format;
        self.configuration.color_space = negotiated.color_space;
        self.texture_view_format = negotiated.texture_view_format();
        self.configuration.view_formats = Vec::from_iter(self.texture_view_format);
        self.resolved_color_space = negotiated.resolved;
    }

    /// Refreshes the supported color spaces from `caps` and renegotiates the
    /// surface when needed. Returns `true` if the negotiation ran.
    ///
    /// It runs when `request_changed`, when the set of supported color spaces
    /// differs from the last configuration, or when `caps` stops listing the
    /// configured color space for the configured format. The capabilities
    /// change at runtime when the OS HDR setting changes. Configuring a lost
    /// color space again fails wgpu validation with
    /// `ConfigureSurfaceError::UnsupportedColorSpace`. The negotiation resolves
    /// `display_target` again, so with [`DisplayTarget::hdr`] the surface moves
    /// to the best supported color space in both directions: down to the next
    /// best when one is lost, and up when a better one appears.
    fn renegotiate(
        &mut self,
        caps: &wgpu::SurfaceCapabilities,
        display_target: &DisplayTarget,
        request_changed: bool,
    ) -> bool {
        let supported = supported_color_spaces(caps);
        let supported_changed = supported != self.supported_color_spaces;
        self.supported_color_spaces = supported;
        let lost = color_space_lost(
            caps,
            self.configuration.format,
            self.configuration.color_space,
        );
        if !(request_changed || supported_changed || lost) {
            return false;
        }
        if lost {
            warn_once!(
                "Surface color space {:?} is not supported for {:?} any more. The OS HDR \
                setting may have changed. Renegotiating the surface.",
                self.configuration.color_space,
                self.configuration.format
            );
        } else if supported_changed {
            info!(
                "Surface color spaces changed to {supported:?}. The OS HDR setting may have \
                changed. Renegotiating the surface."
            );
        }
        self.apply_negotiated(negotiate_surface_format(caps, display_target));
        true
    }
}

/// Returns `true` if `caps` does not list `color_space` for `format`.
///
/// [`wgpu::SurfaceColorSpace::Auto`] has no capabilities flag and is never
/// lost.
fn color_space_lost(
    caps: &wgpu::SurfaceCapabilities,
    format: TextureFormat,
    color_space: wgpu::SurfaceColorSpace,
) -> bool {
    color_space
        .to_color_spaces()
        .is_some_and(|flag| !caps.color_spaces(format).contains(flag))
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
    mut windows: Query<(MainEntity, &mut ExtractedWindow, Option<&mut SurfaceData>)>,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
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

        let Some(mut surface_data) = maybe_surface_data else {
            continue;
        };

        // We didn't present the previous frame, so we can keep using our existing swapchain texture.
        if window.has_swapchain_texture()
            && !window.size_changed
            && !window.present_mode_changed
            && !window.color_space_request_changed
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
            wgpu::CurrentSurfaceTexture::Success(surface_texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                window.set_swapchain_texture(surface_texture, surface_data.texture_view_format);
            }
            #[cfg(target_os = "linux")]
            wgpu::CurrentSurfaceTexture::Timeout if may_erroneously_timeout() => {
                bevy_log::trace!(
                    "Couldn't get swap chain texture. This is probably a quirk \
                        of your Linux GPU driver, so it can be safely ignored."
                );
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                // wgpu reports `Outdated` when the underlying surface changed, which
                // includes its color spaces.
                let caps = surface_data.surface.get_capabilities(&render_adapter);
                if surface_data.renegotiate(&caps, &window.display_target, false) {
                    window.resolved_color_space = Some(surface_data.resolved_color_space);
                }
                let surface = &surface_data.surface;
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
                window.set_swapchain_texture(frame, surface_data.texture_view_format);
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
        if !has_surface_data
            || window.size_changed
            || window.present_mode_changed
            || window.color_space_request_changed
        {
            return true;
        }
    }
    false
}

/// The format and color space [`negotiate_surface_format`] chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NegotiatedSurface {
    format: TextureFormat,
    /// [`wgpu::SurfaceColorSpace::Auto`], or a color space the surface
    /// supports for `format`. Anything else fails wgpu validation.
    color_space: wgpu::SurfaceColorSpace,
    /// The color space the surface presents in.
    resolved: SurfaceColorSpace,
}

impl NegotiatedSurface {
    /// The sRGB view format to render through, or `None` to use the texture as
    /// is.
    ///
    /// Only SDR sRGB gets an sRGB view. An HDR negotiation can choose an
    /// 8-bit format. Values written to it must already be encoded for its
    /// color space, and an sRGB view would encode them again.
    fn texture_view_format(&self) -> Option<TextureFormat> {
        let view_format = self.format.add_srgb_suffix();
        (self.resolved == SurfaceColorSpace::Srgb && view_format != self.format)
            .then_some(view_format)
    }
}

/// The HDR color spaces Bevy tries for [`DisplayTarget::hdr`], best first.
///
/// PQ is the HDR10 signal every HDR display decodes. Linear scRGB is the
/// desktop compositor path. Extended sRGB and extended Display P3 are the
/// web path.
const HDR_PREFERENCE: [SurfaceColorSpace; 4] = [
    SurfaceColorSpace::Pq,
    SurfaceColorSpace::ScRgbLinear,
    SurfaceColorSpace::ExtendedSrgb,
    SurfaceColorSpace::ExtendedDisplayP3,
];

/// Returns the wgpu color space of an HDR `color_space`, and the
/// capabilities flag that reports it.
///
/// [`SurfaceColorSpace::Srgb`] is not an explicit color space: the SDR path
/// configures the surface with [`wgpu::SurfaceColorSpace::Auto`].
fn hdr_color_space(
    color_space: SurfaceColorSpace,
) -> Option<(wgpu::SurfaceColorSpace, wgpu::SurfaceColorSpaces)> {
    let wgpu_color_space = match color_space {
        SurfaceColorSpace::Srgb => return None,
        SurfaceColorSpace::ScRgbLinear => wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
        SurfaceColorSpace::Pq => wgpu::SurfaceColorSpace::Bt2100Pq,
        SurfaceColorSpace::ExtendedSrgb => wgpu::SurfaceColorSpace::ExtendedSrgb,
        SurfaceColorSpace::ExtendedDisplayP3 => wgpu::SurfaceColorSpace::ExtendedDisplayP3,
    };
    let flag = wgpu_color_space
        .to_color_spaces()
        .expect("every explicit surface color space has a capabilities flag");
    Some((wgpu_color_space, flag))
}

/// Returns the first of `preferred` that supports `flag`, else the first
/// non-sRGB format in `caps` that does.
///
/// Values in an HDR color space are already encoded for it, so the format
/// must not add sRGB encoding on write.
fn first_format_in(
    caps: &wgpu::SurfaceCapabilities,
    flag: wgpu::SurfaceColorSpaces,
    preferred: &[TextureFormat],
) -> Option<TextureFormat> {
    let preferred = preferred
        .iter()
        .copied()
        .filter(|&format| caps.color_spaces(format).contains(flag));
    let any = caps
        .format_capabilities
        .iter()
        .filter(|fc| fc.color_spaces.contains(flag) && !fc.format.is_srgb())
        .map(|fc| fc.format);
    preferred.chain(any).next()
}

/// Negotiates a surface in `color_space`, if `caps` supports it.
///
/// [`SurfaceColorSpace::Srgb`] takes the first sRGB format in `caps.formats`,
/// else the first format there, with [`wgpu::SurfaceColorSpace::Auto`]. It
/// is `None` only when `caps.formats` is empty.
///
/// [`SurfaceColorSpace::Pq`] prefers `Rgb10a2Unorm`, then `Rgba16Float`,
/// then any format that supports it. wgpu documents `Rgb10a2Unorm` as the
/// typical format for HDR10. [`SurfaceColorSpace::ScRgbLinear`] needs float
/// precision, so only `Rgba16Float` is tried. The extended sRGB color spaces
/// prefer `Rgba16Float`, then any format that supports them.
fn negotiate_color_space(
    caps: &wgpu::SurfaceCapabilities,
    color_space: SurfaceColorSpace,
) -> Option<NegotiatedSurface> {
    let Some((wgpu_color_space, flag)) = hdr_color_space(color_space) else {
        // Rgba8UnormSrgb and Bgra8UnormSrgb are the only sRGB formats wgpu
        // exposes that we can use for surfaces.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
                )
            })
            .or_else(|| caps.formats.first().copied())?;
        return Some(NegotiatedSurface {
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            resolved: SurfaceColorSpace::Srgb,
        });
    };
    let preferred: &[TextureFormat] = match color_space {
        SurfaceColorSpace::Pq => &[TextureFormat::Rgb10a2Unorm, TextureFormat::Rgba16Float],
        SurfaceColorSpace::ScRgbLinear => {
            return caps
                .color_spaces(TextureFormat::Rgba16Float)
                .contains(flag)
                .then_some(NegotiatedSurface {
                    format: TextureFormat::Rgba16Float,
                    color_space: wgpu_color_space,
                    resolved: color_space,
                });
        }
        _ => &[TextureFormat::Rgba16Float],
    };
    first_format_in(caps, flag, preferred).map(|format| NegotiatedSurface {
        format,
        color_space: wgpu_color_space,
        resolved: color_space,
    })
}

/// Returns the [`SurfaceColorSpace`]s a surface with these capabilities can
/// provide. [`SurfaceColorSpace::Srgb`] is included when the surface has a
/// format for [`wgpu::SurfaceColorSpace::Auto`].
fn supported_color_spaces(caps: &wgpu::SurfaceCapabilities) -> SurfaceColorSpaces {
    core::iter::once(SurfaceColorSpace::Srgb)
        .chain(HDR_PREFERENCE)
        .filter(|&color_space| negotiate_color_space(caps, color_space).is_some())
        .fold(SurfaceColorSpaces::EMPTY, SurfaceColorSpaces::with)
}

/// Resolves the [`SurfaceColorSpace`] for a [`DisplayTarget`] against what
/// the surface supports.
///
/// [`DisplayTarget::color_space_override`] wins: the surface uses it when it
/// supports it, else SDR sRGB with a warning. Otherwise
/// [`DisplayTarget::hdr`] takes the first entry of [`HDR_PREFERENCE`] the
/// surface supports, else SDR sRGB. Without either, SDR sRGB.
fn resolve_color_space(
    caps: &wgpu::SurfaceCapabilities,
    display_target: &DisplayTarget,
) -> SurfaceColorSpace {
    if let Some(requested) = display_target.color_space_override {
        if !requested.is_hdr() || negotiate_color_space(caps, requested).is_some() {
            return requested;
        }
        warn_once!(
            "DisplayTarget::color_space_override requests {requested:?}, but this surface \
            does not support it. The OS HDR setting may be off, or the backend may not \
            support it. Using SDR sRGB."
        );
        return SurfaceColorSpace::Srgb;
    }
    if display_target.hdr {
        if let Some(color_space) = HDR_PREFERENCE
            .into_iter()
            .find(|&color_space| negotiate_color_space(caps, color_space).is_some())
        {
            return color_space;
        }
        info_once!(
            "DisplayTarget::hdr is set, but this surface has no HDR color space. Using SDR \
            sRGB."
        );
    }
    SurfaceColorSpace::Srgb
}

/// Chooses the format and color space for a window surface from its
/// [`DisplayTarget`].
///
/// [`resolve_color_space`] picks the color space. For
/// [`DisplayTarget::hdr`] that is Bevy's choice, in the order of
/// [`HDR_PREFERENCE`]: PQ, then linear scRGB, then extended sRGB, then
/// extended Display P3. The surface is renegotiated when the color spaces it
/// supports change at runtime, so with `hdr` the output moves to the best
/// supported color space in both directions. [`negotiate_color_space`] picks
/// the format.
///
/// When an HDR request resolves to SDR sRGB but the surface lists no format
/// for [`wgpu::SurfaceColorSpace::Auto`], which wgpu documents some drivers
/// do in OS HDR mode, the first HDR color space the surface supports is used
/// instead, with a warning. Configuring those surfaces with `Auto` fails
/// validation.
///
/// # Panics
///
/// Panics if the surface offers no format for the request: no format at all,
/// or no format for `Auto` when the request is not HDR.
fn negotiate_surface_format(
    caps: &wgpu::SurfaceCapabilities,
    display_target: &DisplayTarget,
) -> NegotiatedSurface {
    let color_space = resolve_color_space(caps, display_target);
    if let Some(negotiated) = negotiate_color_space(caps, color_space) {
        return negotiated;
    }
    let requested_hdr = display_target.hdr
        || display_target
            .color_space_override
            .is_some_and(|requested| requested.is_hdr());
    if !requested_hdr {
        panic!("No supported formats for surface");
    }
    let negotiated = HDR_PREFERENCE
        .into_iter()
        .find_map(|color_space| negotiate_color_space(caps, color_space))
        .expect("No supported formats for surface");
    warn_once!(
        "This surface has no format that works with the default color space. Using {:?} \
        in the {:?} color space.",
        negotiated.format,
        negotiated.resolved
    );
    negotiated
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
            let negotiated = negotiate_surface_format(&caps, &window.display_target);
            let supported_color_spaces = supported_color_spaces(&caps);
            let texture_view_format = negotiated.texture_view_format();
            let configuration = SurfaceConfiguration {
                format: negotiated.format,
                color_space: negotiated.color_space,
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
                view_formats: Vec::from_iter(texture_view_format),
            };

            render_device.configure_surface(&surface, &configuration);

            // `SurfaceData` is inserted through commands, so systems later this
            // frame read the color space from the window.
            window.resolved_color_space = Some(negotiated.resolved);
            commands.entity(entity).insert(SurfaceData {
                surface: WgpuSurface::new(surface),
                configuration,
                texture_view_format,
                resolved_color_space: negotiated.resolved,
                supported_color_spaces,
            });
            continue;
        };

        if window.size_changed || window.present_mode_changed || window.color_space_request_changed
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
            data.configuration.present_mode = present_mode(&window, &caps);
            data.renegotiate(
                &caps,
                &window.display_target,
                window.color_space_request_changed,
            );
            render_device.configure_surface(&data.surface, &data.configuration);
        }

        window.resolved_color_space = Some(data.resolved_color_space);
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

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{SurfaceColorSpaces as Flags, SurfaceFormatCapabilities};

    fn fc(format: TextureFormat, color_spaces: Flags) -> SurfaceFormatCapabilities {
        SurfaceFormatCapabilities {
            format,
            color_spaces,
        }
    }

    fn caps(
        formats: Vec<TextureFormat>,
        format_capabilities: Vec<SurfaceFormatCapabilities>,
    ) -> wgpu::SurfaceCapabilities {
        wgpu::SurfaceCapabilities {
            formats,
            format_capabilities,
            ..Default::default()
        }
    }

    /// [`negotiate_surface_format`] for `DisplayTarget { hdr: true }`.
    fn negotiate_hdr(caps: &wgpu::SurfaceCapabilities) -> NegotiatedSurface {
        negotiate_surface_format(
            caps,
            &DisplayTarget {
                hdr: true,
                ..Default::default()
            },
        )
    }

    /// [`negotiate_surface_format`] with `color_space_override` set.
    fn negotiate_override(
        caps: &wgpu::SurfaceCapabilities,
        color_space: SurfaceColorSpace,
    ) -> NegotiatedSurface {
        negotiate_surface_format(
            caps,
            &DisplayTarget {
                color_space_override: Some(color_space),
                ..Default::default()
            },
        )
    }

    /// A Metal-like HDR-capable surface.
    fn metal_like() -> wgpu::SurfaceCapabilities {
        caps(
            vec![
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Bgra8Unorm,
                TextureFormat::Rgba16Float,
                TextureFormat::Rgb10a2Unorm,
            ],
            vec![
                fc(
                    TextureFormat::Bgra8UnormSrgb,
                    Flags::SRGB | Flags::DISPLAY_P3,
                ),
                fc(TextureFormat::Bgra8Unorm, Flags::SRGB | Flags::DISPLAY_P3),
                fc(
                    TextureFormat::Rgba16Float,
                    Flags::SRGB
                        | Flags::DISPLAY_P3
                        | Flags::EXTENDED_SRGB_LINEAR
                        | Flags::EXTENDED_SRGB
                        | Flags::EXTENDED_DISPLAY_P3
                        | Flags::BT2100_PQ
                        | Flags::BT2100_HLG,
                ),
                fc(
                    TextureFormat::Rgb10a2Unorm,
                    Flags::SRGB | Flags::DISPLAY_P3 | Flags::BT2100_PQ | Flags::BT2100_HLG,
                ),
            ],
        )
    }

    /// A browser WebGPU-like surface on an HDR display. `Rgba16Float` supports
    /// the encoded extended sRGB and Display P3 color spaces, but no linear
    /// scRGB and no HDR10.
    fn web_like() -> wgpu::SurfaceCapabilities {
        caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(
                    TextureFormat::Bgra8UnormSrgb,
                    Flags::SRGB | Flags::DISPLAY_P3,
                ),
                fc(
                    TextureFormat::Rgba16Float,
                    Flags::SRGB
                        | Flags::DISPLAY_P3
                        | Flags::EXTENDED_SRGB
                        | Flags::EXTENDED_DISPLAY_P3,
                ),
            ],
        )
    }

    /// A Vulkan-like surface on an HDR display. `Rgb10a2Unorm` supports only
    /// HDR color spaces, so it is absent from `formats`.
    fn vulkan_hdr_like() -> wgpu::SurfaceCapabilities {
        caps(
            vec![
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Bgra8Unorm,
                TextureFormat::Rgba16Float,
            ],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Bgra8Unorm, Flags::SRGB),
                fc(TextureFormat::Rgba16Float, Flags::EXTENDED_SRGB_LINEAR),
                fc(
                    TextureFormat::Rgb10a2Unorm,
                    Flags::BT2100_PQ | Flags::BT2100_HLG,
                ),
            ],
        )
    }

    /// A surface with scRGB but no HDR10.
    fn scrgb_only() -> wgpu::SurfaceCapabilities {
        caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Rgba16Float, Flags::EXTENDED_SRGB_LINEAR),
            ],
        )
    }

    /// A surface whose only HDR color space is extended Display P3.
    fn extended_p3_only() -> wgpu::SurfaceCapabilities {
        caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Rgba16Float, Flags::EXTENDED_DISPLAY_P3),
            ],
        )
    }

    /// A surface with no HDR color spaces.
    fn sdr_only() -> wgpu::SurfaceCapabilities {
        caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Bgra8Unorm],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Bgra8Unorm, Flags::SRGB),
            ],
        )
    }

    fn sdr(format: TextureFormat) -> NegotiatedSurface {
        NegotiatedSurface {
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            resolved: SurfaceColorSpace::Srgb,
        }
    }

    fn pq(format: TextureFormat) -> NegotiatedSurface {
        NegotiatedSurface {
            format,
            color_space: wgpu::SurfaceColorSpace::Bt2100Pq,
            resolved: SurfaceColorSpace::Pq,
        }
    }

    const SCRGB_LINEAR: NegotiatedSurface = NegotiatedSurface {
        format: TextureFormat::Rgba16Float,
        color_space: wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
        resolved: SurfaceColorSpace::ScRgbLinear,
    };

    const EXTENDED_SRGB: NegotiatedSurface = NegotiatedSurface {
        format: TextureFormat::Rgba16Float,
        color_space: wgpu::SurfaceColorSpace::ExtendedSrgb,
        resolved: SurfaceColorSpace::ExtendedSrgb,
    };

    const EXTENDED_P3: NegotiatedSurface = NegotiatedSurface {
        format: TextureFormat::Rgba16Float,
        color_space: wgpu::SurfaceColorSpace::ExtendedDisplayP3,
        resolved: SurfaceColorSpace::ExtendedDisplayP3,
    };

    #[test]
    fn default_selects_an_srgb_format_with_auto() {
        let default = DisplayTarget::default();
        assert_eq!(
            negotiate_surface_format(&metal_like(), &default),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            negotiate_surface_format(&sdr_only(), &default),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            negotiate_surface_format(
                &caps(
                    vec![TextureFormat::Bgra8Unorm, TextureFormat::Rgba16Float],
                    vec![]
                ),
                &default
            ),
            sdr(TextureFormat::Bgra8Unorm)
        );
        assert_eq!(
            negotiate_surface_format(
                &caps(
                    vec![TextureFormat::Rgba8UnormSrgb, TextureFormat::Bgra8UnormSrgb],
                    vec![]
                ),
                &default
            ),
            sdr(TextureFormat::Rgba8UnormSrgb)
        );
    }

    #[test]
    fn only_sdr_gets_an_srgb_view_format() {
        assert_eq!(
            sdr(TextureFormat::Bgra8Unorm).texture_view_format(),
            Some(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            sdr(TextureFormat::Bgra8UnormSrgb).texture_view_format(),
            None
        );
        assert_eq!(pq(TextureFormat::Bgra8Unorm).texture_view_format(), None);
        assert_eq!(SCRGB_LINEAR.texture_view_format(), None);
    }

    #[test]
    fn hdr_takes_the_first_supported_color_space_in_preference_order() {
        assert_eq!(
            negotiate_hdr(&metal_like()),
            pq(TextureFormat::Rgb10a2Unorm)
        );
        assert_eq!(
            negotiate_hdr(&vulkan_hdr_like()),
            pq(TextureFormat::Rgb10a2Unorm)
        );
        assert_eq!(negotiate_hdr(&scrgb_only()), SCRGB_LINEAR);
        assert_eq!(negotiate_hdr(&web_like()), EXTENDED_SRGB);
        assert_eq!(negotiate_hdr(&extended_p3_only()), EXTENDED_P3);
    }

    #[test]
    fn hdr_without_an_hdr_color_space_is_sdr() {
        assert_eq!(
            negotiate_hdr(&sdr_only()),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn override_wins_over_hdr() {
        let target = DisplayTarget {
            hdr: true,
            color_space_override: Some(SurfaceColorSpace::ScRgbLinear),
            ..Default::default()
        };
        assert_eq!(
            negotiate_surface_format(&metal_like(), &target),
            SCRGB_LINEAR
        );

        let target = DisplayTarget {
            hdr: true,
            color_space_override: Some(SurfaceColorSpace::Srgb),
            ..Default::default()
        };
        assert_eq!(
            negotiate_surface_format(&metal_like(), &target),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn scrgb_picks_rgba16float_with_extended_srgb_linear() {
        assert_eq!(
            negotiate_override(&metal_like(), SurfaceColorSpace::ScRgbLinear),
            SCRGB_LINEAR
        );
        assert_eq!(
            negotiate_override(&vulkan_hdr_like(), SurfaceColorSpace::ScRgbLinear),
            SCRGB_LINEAR
        );
    }

    #[test]
    fn scrgb_requires_the_color_space_not_just_the_format() {
        // `Rgba16Float` is listed, but only in the sRGB color space, where
        // linear scRGB values would display incorrectly.
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Rgba16Float, Flags::SRGB),
            ],
        );
        assert_eq!(
            negotiate_override(&caps, SurfaceColorSpace::ScRgbLinear),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn pq_prefers_rgb10a2unorm() {
        assert_eq!(
            negotiate_override(&vulkan_hdr_like(), SurfaceColorSpace::Pq),
            pq(TextureFormat::Rgb10a2Unorm)
        );
        // Metal-like: both formats advertise HDR10, and Rgb10a2Unorm is chosen
        // even though Rgba16Float is listed first.
        assert_eq!(
            negotiate_override(&metal_like(), SurfaceColorSpace::Pq),
            pq(TextureFormat::Rgb10a2Unorm)
        );
    }

    #[test]
    fn pq_uses_rgba16float_when_it_is_the_only_hdr10_format() {
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(
                    TextureFormat::Rgba16Float,
                    Flags::EXTENDED_SRGB_LINEAR | Flags::BT2100_PQ,
                ),
            ],
        );
        assert_eq!(
            negotiate_override(&caps, SurfaceColorSpace::Pq),
            pq(TextureFormat::Rgba16Float)
        );
    }

    #[test]
    fn pq_takes_any_hdr10_format_when_preferred_ones_are_missing() {
        // A driver advertising HDR10 on an 8-bit format only.
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Bgra8Unorm],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Bgra8Unorm, Flags::SRGB | Flags::BT2100_PQ),
            ],
        );
        assert_eq!(
            negotiate_override(&caps, SurfaceColorSpace::Pq),
            pq(TextureFormat::Bgra8Unorm)
        );
    }

    #[test]
    fn extended_srgb_and_display_p3_are_distinct_color_spaces() {
        assert_eq!(
            negotiate_override(&web_like(), SurfaceColorSpace::ExtendedSrgb),
            EXTENDED_SRGB
        );
        assert_eq!(
            negotiate_override(&web_like(), SurfaceColorSpace::ExtendedDisplayP3),
            EXTENDED_P3
        );
        assert_eq!(
            negotiate_override(&metal_like(), SurfaceColorSpace::ExtendedSrgb),
            EXTENDED_SRGB
        );
        // Neither stands in for the other.
        assert_eq!(
            negotiate_override(&extended_p3_only(), SurfaceColorSpace::ExtendedSrgb),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        let extended_srgb_only = caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                fc(TextureFormat::Bgra8UnormSrgb, Flags::SRGB),
                fc(TextureFormat::Rgba16Float, Flags::EXTENDED_SRGB),
            ],
        );
        assert_eq!(
            negotiate_override(&extended_srgb_only, SurfaceColorSpace::ExtendedDisplayP3),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn an_unsupported_override_is_sdr_not_another_hdr_color_space() {
        assert_eq!(
            negotiate_override(&scrgb_only(), SurfaceColorSpace::Pq),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            negotiate_override(&web_like(), SurfaceColorSpace::Pq),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            negotiate_override(&web_like(), SurfaceColorSpace::ScRgbLinear),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            negotiate_override(&sdr_only(), SurfaceColorSpace::ExtendedSrgb),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    /// A driver in OS HDR mode that lists formats only in explicit color
    /// spaces.
    fn pq_only() -> wgpu::SurfaceCapabilities {
        caps(
            vec![],
            vec![fc(TextureFormat::Rgb10a2Unorm, Flags::BT2100_PQ)],
        )
    }

    #[test]
    fn empty_auto_formats_fall_back_to_an_explicit_color_space_for_hdr_requests() {
        assert_eq!(negotiate_hdr(&pq_only()), pq(TextureFormat::Rgb10a2Unorm));
        assert_eq!(
            negotiate_override(&pq_only(), SurfaceColorSpace::ScRgbLinear),
            pq(TextureFormat::Rgb10a2Unorm)
        );

        let extended_p3_only = caps(
            vec![],
            vec![fc(TextureFormat::Rgba16Float, Flags::EXTENDED_DISPLAY_P3)],
        );
        assert_eq!(negotiate_hdr(&extended_p3_only), EXTENDED_P3);
    }

    #[test]
    #[should_panic(expected = "No supported formats for surface")]
    fn empty_auto_formats_panic_for_a_default_request() {
        negotiate_surface_format(&pq_only(), &DisplayTarget::default());
    }

    #[test]
    #[should_panic(expected = "No supported formats for surface")]
    fn empty_auto_formats_panic_for_an_srgb_override() {
        negotiate_override(&pq_only(), SurfaceColorSpace::Srgb);
    }

    #[test]
    #[should_panic(expected = "No supported formats for surface")]
    fn no_formats_at_all_panics() {
        negotiate_surface_format(&caps(vec![], vec![]), &DisplayTarget::default());
    }

    #[test]
    fn srgb_is_supported_only_with_a_format_for_auto() {
        assert!(supported_color_spaces(&sdr_only()).contains(SurfaceColorSpace::Srgb));
        assert!(supported_color_spaces(&pq_only())
            .iter()
            .eq([SurfaceColorSpace::Pq]));
    }

    #[test]
    fn a_lost_color_space_renegotiates_to_the_next_best() {
        let pq_surface = pq(TextureFormat::Rgb10a2Unorm);

        // The OS HDR setting is turned off: HDR10 is gone, and `hdr` falls
        // back to SDR.
        assert!(color_space_lost(
            &sdr_only(),
            pq_surface.format,
            pq_surface.color_space
        ));
        assert_eq!(
            negotiate_hdr(&sdr_only()),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );

        // HDR10 is gone but scRGB remains: `hdr` moves to the next best.
        assert!(color_space_lost(
            &scrgb_only(),
            pq_surface.format,
            pq_surface.color_space
        ));
        assert_eq!(negotiate_hdr(&scrgb_only()), SCRGB_LINEAR);

        // An override does not move to another HDR color space.
        assert_eq!(
            negotiate_override(&scrgb_only(), SurfaceColorSpace::Pq),
            sdr(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn a_supported_color_space_is_not_lost() {
        let pq_surface = pq(TextureFormat::Rgb10a2Unorm);
        assert!(!color_space_lost(
            &metal_like(),
            pq_surface.format,
            pq_surface.color_space
        ));
        assert!(!color_space_lost(
            &scrgb_only(),
            SCRGB_LINEAR.format,
            SCRGB_LINEAR.color_space
        ));
    }

    #[test]
    fn auto_is_never_lost() {
        let sdr_surface = sdr(TextureFormat::Bgra8UnormSrgb);
        for fixture in [metal_like(), sdr_only(), pq_only(), caps(vec![], vec![])] {
            assert!(!color_space_lost(
                &fixture,
                sdr_surface.format,
                sdr_surface.color_space
            ));
        }
    }

    #[test]
    fn only_hdr_and_override_changes_need_renegotiation() {
        let default = DisplayTarget::default();
        let hdr = DisplayTarget {
            hdr: true,
            ..Default::default()
        };
        let pq_override = DisplayTarget {
            color_space_override: Some(SurfaceColorSpace::Pq),
            ..Default::default()
        };
        let scrgb_override = DisplayTarget {
            color_space_override: Some(SurfaceColorSpace::ScRgbLinear),
            ..Default::default()
        };
        let calibrated = DisplayTarget {
            paper_white_nits: Some(300.0),
            peak_luminance_nits: Some(1500.0),
            min_luminance_nits: Some(0.01),
            ..Default::default()
        };

        assert!(color_space_request_changed(&default, &hdr));
        assert!(color_space_request_changed(&pq_override, &scrgb_override));
        assert!(color_space_request_changed(&pq_override, &default));
        assert!(!color_space_request_changed(&default, &calibrated));
        assert!(!color_space_request_changed(&default, &default));
        assert!(!color_space_request_changed(&pq_override, &pq_override));
    }

    #[test]
    fn supported_color_spaces_matches_negotiable_set() {
        use SurfaceColorSpace::{ExtendedDisplayP3, ExtendedSrgb, Pq, ScRgbLinear, Srgb};

        let cases: [(
            &str,
            fn() -> wgpu::SurfaceCapabilities,
            Vec<SurfaceColorSpace>,
        ); 6] = [
            (
                "metal_like",
                metal_like,
                vec![Srgb, ScRgbLinear, Pq, ExtendedSrgb, ExtendedDisplayP3],
            ),
            (
                "web_like",
                web_like,
                vec![Srgb, ExtendedSrgb, ExtendedDisplayP3],
            ),
            (
                "vulkan_hdr_like",
                vulkan_hdr_like,
                vec![Srgb, ScRgbLinear, Pq],
            ),
            ("scrgb_only", scrgb_only, vec![Srgb, ScRgbLinear]),
            (
                "extended_p3_only",
                extended_p3_only,
                vec![Srgb, ExtendedDisplayP3],
            ),
            ("sdr_only", sdr_only, vec![Srgb]),
        ];

        for (name, fixture, expected) in cases {
            assert_eq!(
                supported_color_spaces(&fixture())
                    .iter()
                    .collect::<Vec<_>>(),
                expected,
                "{name}"
            );
        }
    }
}
