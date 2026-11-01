//! Resolves the [`DisplayTarget`] each view encodes for.
//!
//! Windows have it as a required component. Other render targets register
//! one in [`ManualDisplayTargets`].

use bevy_camera::NormalizedRenderTarget;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_extract_macros::ExtractResource;
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_window::{DisplayTarget, ResolvedDisplayTarget, SurfaceColorSpace};

use super::window::ExtractedWindow;
use crate::{camera::ExtractedCamera, sync_world::MainEntity, RenderApp};

/// Resource that stores the [`DisplayTarget`] of each render target that is
/// not a [`Window`](bevy_window::Window), keyed by [`NormalizedRenderTarget`].
///
/// Insert into it from the main world. These targets have no surface to
/// negotiate with, so [`DisplayTarget::color_space_override`] is used as is
/// and [`DisplayTarget::hdr`] alone cannot be honored: a target with `hdr`
/// and no override resolves to [`SurfaceColorSpace::Srgb`].
#[derive(Default, Clone, Debug, PartialEq, Resource, ExtractResource, Reflect, Deref, DerefMut)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[extract_app(RenderApp)]
pub struct ManualDisplayTargets(HashMap<NormalizedRenderTarget, DisplayTarget>);

/// The [`ResolvedDisplayTarget`] a view encodes for, after surface
/// negotiation.
///
/// Required by [`ExtractedCamera`]. A target that cannot be resolved gets
/// the default: SDR sRGB at 100 nits.
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, Default)]
pub struct ViewDisplayTarget(pub ResolvedDisplayTarget);

impl ViewDisplayTarget {
    /// Returns `true` if the color space has high dynamic range. See
    /// [`SurfaceColorSpace::is_hdr`].
    pub fn is_hdr(&self) -> bool {
        self.0.color_space.is_hdr()
    }
}

/// Resolves a [`DisplayTarget`] that has no surface to negotiate with.
fn resolve_manual_display_target(display_target: &DisplayTarget) -> ResolvedDisplayTarget {
    display_target.resolve(
        display_target
            .color_space_override
            .unwrap_or(SurfaceColorSpace::Srgb),
    )
}

/// Resolves the [`ViewDisplayTarget`] for a render target.
///
/// A window resolves [`ExtractedWindow::display_target`] with the color
/// space the surface negotiated in the previous frame, or with
/// [`SurfaceColorSpace::Srgb`] before the surface is configured. An image or
/// texture view looks up
/// [`ManualDisplayTargets`] by the whole [`NormalizedRenderTarget`], so an
/// image entry must also match the scale factor. Anything else, including a
/// missing entry, is [`DisplayTarget::default`] resolved as SDR sRGB.
pub fn resolve_view_display_target<'a>(
    target: Option<&NormalizedRenderTarget>,
    windows: impl IntoIterator<Item = (Entity, &'a ExtractedWindow)>,
    manual_display_targets: &ManualDisplayTargets,
) -> ViewDisplayTarget {
    let resolved = match target {
        Some(NormalizedRenderTarget::Window(window_ref)) => windows
            .into_iter()
            .find(|(entity, _)| *entity == window_ref.entity())
            .map(|(_, window)| {
                window.display_target.resolve(
                    window
                        .resolved_color_space
                        .unwrap_or(SurfaceColorSpace::Srgb),
                )
            }),
        Some(
            target @ (NormalizedRenderTarget::Image(_) | NormalizedRenderTarget::TextureView(_)),
        ) => manual_display_targets
            .get(target)
            .map(resolve_manual_display_target),
        Some(NormalizedRenderTarget::None { .. }) | None => None,
    };
    ViewDisplayTarget(resolved.unwrap_or_default())
}

/// Updates each view's [`ViewDisplayTarget`] for the color space each window
/// surface is configured with this frame.
pub(crate) fn prepare_view_display_targets(
    windows: Query<(MainEntity, &ExtractedWindow)>,
    manual_display_targets: Res<ManualDisplayTargets>,
    mut views: Query<(&ExtractedCamera, &mut ViewDisplayTarget)>,
) {
    for (camera, mut view_display_target) in &mut views {
        view_display_target.set_if_neq(resolve_view_display_target(
            camera.target.as_ref(),
            windows.iter(),
            &manual_display_targets,
        ));
    }
}
