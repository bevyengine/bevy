//! Reads what each window's display reports, resolves the window's
//! [`DisplayTarget`] with it, and writes both back to the main world.

use bevy_camera::NormalizedRenderTarget;
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_platform::time::Instant;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_window::{
    DisplayTarget, ResolvedDisplayTarget, SurfaceColorSpace, WindowSurfaceColorSpaces,
    MAX_LUMINANCE_NITS,
};
use core::time::Duration;

use crate::camera::ExtractedCamera;
use crate::render_resource::DisplayHdrInfo;
use crate::renderer::RenderAdapter;
use crate::sync_world::MainEntity;
use crate::view::ViewDisplayTarget;
use crate::MainWorld;

use super::{ExtractedWindow, SurfaceData};

/// What a window's display reports about its HDR state and luminance.
///
/// The renderer reads it with wgpu's [`Surface::display_hdr_info`] and
/// inserts it on the window once the display reports something. Writing to it
/// has no effect. A field is `None` when the platform doesn't report it.
///
/// [`Surface::display_hdr_info`]: wgpu::Surface::display_hdr_info
#[derive(Component, Debug, Clone, Default, PartialEq, Deref, Reflect)]
#[reflect(opaque)]
#[reflect(Component, Default, Debug, Clone, PartialEq)]
pub struct WindowDisplayHdrInfo(pub DisplayHdrInfo);

/// Returns `true` if the display reports that the OS HDR setting is off.
///
/// Only checked on Windows. On macOS the flag is also `false` whenever the
/// display has no headroom at that moment, which doesn't mean HDR is off.
pub(super) fn display_reports_sdr(info: Option<&DisplayHdrInfo>) -> bool {
    cfg!(target_os = "windows")
        && info.and_then(|info| info.coarse?.high_dynamic_range) == Some(false)
}

/// The relative change below which a new reading counts as unchanged, so
/// noise does not trigger change detection in the main world.
const CHANGE_TOLERANCE: f32 = 0.01;

/// Returns `true` if `new` differs from `old` by more than
/// [`CHANGE_TOLERANCE`], or only one of them is reported.
fn changed_beyond_tolerance(old: Option<f32>, new: Option<f32>) -> bool {
    match (old, new) {
        (Some(a), Some(b)) => (a - b).abs() > CHANGE_TOLERANCE * a.abs().max(f32::MIN_POSITIVE),
        (None, None) => false,
        _ => true,
    }
}

/// Reads what the display of `surface` reports, or `None` if it reports
/// nothing. Non-finite values become `None`, since a NaN never compares equal
/// and would count as a change on every read.
pub(super) fn read_display(
    surface: &wgpu::Surface,
    adapter: &RenderAdapter,
) -> Option<DisplayHdrInfo> {
    let mut info = surface.display_hdr_info(adapter);
    let finite = |value: &mut Option<f32>| *value = value.filter(|v| v.is_finite());
    if let Some(l) = &mut info.luminance {
        for value in [
            &mut l.max_nits,
            &mut l.max_full_frame_nits,
            &mut l.min_nits,
            &mut l.sdr_white_nits,
        ] {
            finite(value);
        }
    }
    if let Some(h) = &mut info.headroom {
        for value in [&mut h.current, &mut h.potential, &mut h.reference] {
            finite(value);
        }
    }
    if let Some(c) = &mut info.chromaticity {
        for xy in [&mut c.red, &mut c.green, &mut c.blue, &mut c.white] {
            *xy = xy.filter(|xy| xy.iter().all(|v| v.is_finite()));
        }
    }
    (info != DisplayHdrInfo::default()).then_some(info)
}

/// Returns `true` if `new` should replace `old`. The headroom and SDR white
/// change with the display brightness, so they must change by more than
/// [`CHANGE_TOLERANCE`]. Any other change counts.
fn reading_changed(old: &DisplayHdrInfo, new: &DisplayHdrInfo) -> bool {
    let sdr_white = |info: &DisplayHdrInfo| info.luminance.and_then(|l| l.sdr_white_nits);
    let without_drift = |info: &DisplayHdrInfo| {
        let mut info = info.clone();
        if let Some(h) = &mut info.headroom {
            h.current = None;
        }
        if let Some(l) = &mut info.luminance {
            l.sdr_white_nits = None;
        }
        info
    };
    changed_beyond_tolerance(old.tone_map_headroom(), new.tone_map_headroom())
        || changed_beyond_tolerance(sdr_white(old), sdr_white(new))
        || without_drift(old) != without_drift(new)
}

impl SurfaceData {
    /// Reads the display again. A reading with nothing in it keeps the last
    /// one, since the read fails for a moment while a window moves between
    /// screens.
    fn read_display(&mut self, adapter: &RenderAdapter) {
        self.display_read_at = Instant::now();
        let Some(info) = read_display(&self.surface, adapter) else {
            return;
        };
        if self
            .display_hdr_info
            .as_ref()
            .is_none_or(|old| reading_changed(old, &info))
        {
            self.display_hdr_info = Some(info);
        }
    }
}

/// Resolves a window's [`DisplayTarget`] for its surface color space and
/// what its display reports. A luminance the app set takes precedence over a
/// reported one.
pub(super) fn resolve_display_target(
    target: &DisplayTarget,
    color_space: SurfaceColorSpace,
    info: Option<&DisplayHdrInfo>,
) -> ResolvedDisplayTarget {
    let positive = |value: &f32| *value > 0.0;
    let luminance = info.and_then(|info| info.luminance).unwrap_or_default();
    // A reported value is clamped without the warning a calibrated one gets.
    let clamp = |value: f32| value.min(MAX_LUMINANCE_NITS);
    let mut resolved = DisplayTarget {
        // SDR output keeps its default paper white. Windows reports an SDR
        // white of 80 nits whenever OS HDR is off.
        paper_white_nits: target.paper_white_nits.or(luminance
            .sdr_white_nits
            .filter(|v| color_space.is_hdr() && positive(v))
            .map(clamp)),
        min_luminance_nits: target
            .min_luminance_nits
            .or(luminance.min_nits.filter(|v| *v >= 0.0).map(clamp)),
        ..*target
    }
    .resolve(color_space);

    // The peak from the headroom needs the resolved paper white.
    if target.peak_luminance_nits.is_none() && color_space.is_hdr() {
        let paper_white = resolved.paper_white_nits;
        let peak = luminance.max_nits.filter(positive).or_else(|| {
            let headroom = info.and_then(DisplayHdrInfo::tone_map_headroom)?;
            // The product can overflow to infinity.
            Some(paper_white * headroom).filter(|v| v.is_finite() && *v > 0.0)
        });
        if let Some(peak) = peak {
            resolved.peak_luminance_nits = clamp(peak).max(paper_white);
        }
    }
    resolved
}

/// How often a window is read again on Windows. Turning the OS HDR setting on
/// or off sends the app no event there.
const WINDOWS_READ_INTERVAL: Duration = Duration::from_secs(1);

/// Reads each window's display again after the color space request
/// changes or [`ExtractedWindow::display_changed`] is set.
///
/// Some changes send no event. On macOS the headroom changes with ambient
/// light and battery, so an HDR surface with an uncalibrated luminance is read
/// every frame. On Windows the OS HDR setting and the SDR brightness can
/// change, so a window that requests [`DisplayTarget::hdr`] or has an HDR
/// surface is read every [`WINDOWS_READ_INTERVAL`].
///
/// On Apple platforms this runs on the main thread, because the Metal backend
/// returns nothing from any other thread.
pub(super) fn read_display_states(
    #[cfg(any(target_os = "macos", target_os = "ios"))] _marker: bevy_ecs::system::NonSendMarker,
    mut windows: Query<(&ExtractedWindow, &mut SurfaceData)>,
    render_adapter: Res<RenderAdapter>,
) {
    for (window, mut surface_data) in &mut windows {
        let target = &window.display_target;
        let hdr_surface = surface_data.resolved_color_space.is_hdr();
        let uncalibrated = target.paper_white_nits.is_none()
            || target.peak_luminance_nits.is_none()
            || target.min_luminance_nits.is_none();
        let read_every_frame = cfg!(target_os = "macos") && hdr_surface && uncalibrated;
        let read_on_interval = cfg!(target_os = "windows")
            && (target.hdr || hdr_surface)
            && surface_data.display_read_at.elapsed() >= WINDOWS_READ_INTERVAL;
        if window.color_space_request_changed
            || window.display_changed
            || read_every_frame
            || read_on_interval
        {
            surface_data.read_display(&render_adapter);
        }
    }
}

/// Updates each view's [`ViewDisplayTarget`] for the color space each window
/// surface is configured with this frame, and what its display reports.
pub(super) fn prepare_view_display_targets(
    windows: Query<(MainEntity, &ExtractedWindow, &SurfaceData)>,
    mut views: Query<(&ExtractedCamera, &mut ViewDisplayTarget)>,
) {
    for (main_entity, window, surface_data) in &windows {
        let resolved = ViewDisplayTarget(resolve_display_target(
            &window.display_target,
            surface_data.resolved_color_space,
            surface_data.display_hdr_info.as_ref(),
        ));
        for (camera, mut view_display_target) in &mut views {
            if matches!(
                &camera.target,
                Some(NormalizedRenderTarget::Window(window_ref)) if window_ref.entity() == main_entity
            ) {
                view_display_target.set_if_neq(resolved);
            }
        }
    }
}

/// Writes each window's [`WindowSurfaceColorSpaces`],
/// [`ResolvedDisplayTarget`] and [`WindowDisplayHdrInfo`] back to the main
/// world.
pub(super) fn write_back_display_state(
    mut main_world: ResMut<MainWorld>,
    windows: Query<(MainEntity, &ExtractedWindow, &SurfaceData)>,
) {
    for (main_entity, window, surface_data) in windows.iter() {
        super::insert_on_change(
            &mut main_world,
            main_entity,
            WindowSurfaceColorSpaces {
                resolved: surface_data.resolved_color_space,
                supported: surface_data.supported_color_spaces,
            },
        );
        super::insert_on_change(&mut main_world, main_entity, window.resolved_display_target);
        if let Some(info) = &surface_data.display_hdr_info {
            super::insert_on_change(
                &mut main_world,
                main_entity,
                WindowDisplayHdrInfo(info.clone()),
            );
        }
    }
}
