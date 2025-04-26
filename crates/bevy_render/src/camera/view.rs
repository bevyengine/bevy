use bevy_ecs::{
    component::{Component, HookContext},
    world::DeferredWorld,
};
use bevy_math::{Rect, URect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use tracing::warn;

use core::ops::Range;
use std::sync::Arc;

use crate::{primitives::SubRect, sync_world::SyncToRenderWorld};

use super::{
    CompositedBy, CompositorEvent, NormalizedRenderTarget, RenderGraphDriver, RenderTargetInfo,
};

#[derive(Copy, Clone, Default, Debug, Component, Reflect)]
#[component(immutable, on_insert = Self::on_insert, on_remove = trigger_view_changed)]
#[require(RenderGraphDriver, SyncToRenderWorld)]
pub enum View {
    Disabled,
    #[default]
    Enabled,
}

impl View {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    fn on_insert(world: DeferredWorld, ctx: HookContext) {
        if world.entity(ctx.entity).get::<CompositedBy>().is_none() {
            warn!(
                concat!(
                    "{}Entity {} has a View component, but it doesn't have a compositor configured.",
                    "Consider adding a `CompositedBy` component that points to an entity with a Compositor."
                ),
                ctx.caller.map(|location| format!("{location}: ")).unwrap_or_default(), ctx.entity,
            );
        }

        trigger_view_changed(world, ctx);
    }
}

fn trigger_view_changed(mut world: DeferredWorld, ctx: HookContext) {
    world.trigger_targets(CompositorEvent::ViewChanged(ctx.entity), ctx.entity);
}

/// Settings to define the area of a render target to
/// render a view to.
///
/// See [`SubRect`] for more info.
#[derive(Debug, Component, Clone, Reflect, PartialEq)]
#[component(
    immutable,
    on_insert = trigger_view_changed,
    on_remove = trigger_view_changed
)]
#[reflect(Clone, PartialEq, Default)]
pub struct SubView {
    /// The sub-rectangle within which to render the view.
    pub sub_rect: Option<SubRect>,
    /// The minimum and maximum depth to render (on a scale from 0.0 to 1.0).
    pub depth: Range<f32>,
}

impl SubView {
    pub fn get_viewport(&self, physical_size: UVec2) -> Viewport {
        let viewport_rect = self
            .sub_rect
            .unwrap_or_default()
            .scaled_roughly_to(physical_size);
        Viewport {
            physical_position: viewport_rect.offset.as_uvec2(),
            physical_size: viewport_rect.size,
            depth: self.depth.clone(),
        }
    }

    fn on_insert(world: DeferredWorld, ctx: HookContext) {
        let sub_view = world.entity(ctx.entity).get::<SubView>().unwrap();

        //TODO: more general handling. Maybe defer to `Viewport`?

        if sub_view
            .sub_rect
            .is_some_and(|sub_rect| sub_rect.is_empty())
        {
            warn!(
                concat!(
                    "{}Entity {} has a SubView component whose `size` or `full_size` are zero",
                    "in at least one axis. All zero values will be reset to 1."
                ),
                ctx.caller
                    .map(|location| format!("{location}: "))
                    .unwrap_or_default(),
                ctx.entity,
            );
        }
    }
}

impl Default for SubView {
    fn default() -> Self {
        Self {
            sub_rect: Default::default(),
            depth: 0.0..1.0,
        }
    }
}

/// Render viewport configuration for the [`Camera`] component.
///
/// The viewport defines the area on the render target to which the camera renders its image.
/// You can overlay multiple cameras in a single window using viewports to create effects like
/// split screen, minimaps, and character viewers.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default, Clone)]
pub struct Viewport {
    /// The physical position to render this viewport to within the [`RenderTarget`] of this [`Camera`].
    /// (0,0) corresponds to the top-left corner
    pub physical_position: UVec2,
    /// The physical size of the viewport rectangle to render to within the [`RenderTarget`] of this [`Camera`].
    /// The origin of the rectangle is in the top-left corner.
    pub physical_size: UVec2,
    /// The minimum and maximum depth to render (on a scale from 0.0 to 1.0).
    pub depth: Range<f32>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            physical_position: Default::default(),
            physical_size: UVec2::new(1, 1),
            depth: 0.0..1.0,
        }
    }
}

impl Viewport {
    /// Cut the viewport rectangle so that it lies inside a rectangle of the
    /// given size.
    ///
    /// If either of the viewport's position coordinates lies outside the given
    /// dimensions, it will be moved just inside first. If either of the given
    /// dimensions is zero, the position and size of the viewport rectangle will
    /// both be set to zero in that dimension.
    pub fn clamp_to_size(&mut self, size: UVec2) {
        // If the origin of the viewport rect is outside, then adjust so that
        // it's just barely inside. Then, cut off the part that is outside.
        if self.physical_size.x + self.physical_position.x > size.x {
            if self.physical_position.x < size.x {
                self.physical_size.x = size.x - self.physical_position.x;
            } else if size.x > 0 {
                self.physical_position.x = size.x - 1;
                self.physical_size.x = 1;
            } else {
                self.physical_position.x = 0;
                self.physical_size.x = 0;
            }
        }
        if self.physical_size.y + self.physical_position.y > size.y {
            if self.physical_position.y < size.y {
                self.physical_size.y = size.y - self.physical_position.y;
            } else if size.y > 0 {
                self.physical_position.y = size.y - 1;
                self.physical_size.y = 1;
            } else {
                self.physical_position.y = 0;
                self.physical_size.y = 0;
            }
        }
    }
}

#[derive(Component, Clone)]
pub struct ViewTarget {
    pub(crate) target: Arc<(NormalizedRenderTarget, RenderTargetInfo)>,
    pub(crate) viewport: Option<Viewport>,
}

//TODO: UPDATE METHOD DOCS
impl ViewTarget {
    #[inline]
    pub fn target(&self) -> &NormalizedRenderTarget {
        &self.target.0
    }

    #[inline]
    pub fn target_info(&self) -> &RenderTargetInfo {
        &self.target.1
    }

    #[inline]
    pub fn viewport(&self) -> Option<&Viewport> {
        self.viewport.as_ref()
    }

    /// Converts a physical size in this `Camera` to a logical size.
    #[inline]
    pub fn to_logical(&self, physical_size: UVec2) -> Vec2 {
        let scale = self.target_info().scale_factor;
        physical_size.as_vec2() / scale
    }

    /// The rendered physical bounds [`URect`] of the camera. If the `viewport` field is
    /// set to [`Some`], this will be the rect of that custom viewport. Otherwise it will default to
    /// the full physical rect of the current [`RenderTarget`].
    #[inline]
    pub fn physical_viewport_rect(&self) -> URect {
        let min = self
            .viewport
            .as_ref()
            .map(|v| v.physical_position)
            .unwrap_or(UVec2::ZERO);
        let max = min + self.physical_viewport_size();
        URect { min, max }
    }

    /// The rendered logical bounds [`Rect`] of the camera. If the `viewport` field is set to
    /// [`Some`], this will be the rect of that custom viewport. Otherwise it will default to the
    /// full logical rect of the current [`RenderTarget`].
    #[inline]
    pub fn logical_viewport_rect(&self) -> Rect {
        let URect { min, max } = self.physical_viewport_rect();
        Rect {
            min: self.to_logical(min),
            max: self.to_logical(max),
        }
    }

    /// The logical size of this camera's viewport. If the `viewport` field is set to [`Some`], this
    /// will be the size of that custom viewport. Otherwise it will default to the full logical size
    /// of the current [`RenderTarget`].
    ///
    /// For logic that requires the full logical size of the
    /// [`RenderTarget`], prefer [`Camera::logical_target_size`].
    #[inline]
    pub fn logical_viewport_size(&self) -> Vec2 {
        self.viewport
            .as_ref()
            .map(|v| self.to_logical(v.physical_size))
            .unwrap_or(self.logical_target_size())
    }

    /// The physical size of this camera's viewport (in physical pixels).
    /// If the `viewport` field is set to [`Some`], this
    /// will be the size of that custom viewport. Otherwise it will default to the full physical size of
    /// the current [`RenderTarget`].
    /// For logic that requires the full physical size of the [`RenderTarget`], prefer [`Camera::physical_target_size`].
    #[inline]
    pub fn physical_viewport_size(&self) -> UVec2 {
        self.viewport
            .as_ref()
            .map(|v| v.physical_size)
            .unwrap_or(self.physical_target_size())
    }

    /// The full logical size of this camera's [`RenderTarget`], ignoring custom `viewport` configuration.
    /// Note that if the `viewport` field is [`Some`], this will not represent the size of the rendered area.
    /// For logic that requires the size of the actually rendered area, prefer [`Camera::logical_viewport_size`].
    #[inline]
    pub fn logical_target_size(&self) -> Vec2 {
        self.to_logical(self.target_info().physical_size)
    }

    /// The full physical size of this camera's [`RenderTarget`] (in physical pixels),
    /// ignoring custom `viewport` configuration.
    /// Note that if the `viewport` field is [`Some`], this will not represent the size of the rendered area.
    /// For logic that requires the size of the actually rendered area, prefer [`Camera::physical_viewport_size`].
    #[inline]
    pub fn physical_target_size(&self) -> UVec2 {
        self.target_info().physical_size
    }

    #[inline]
    pub fn target_scaling_factor(&self) -> f32 {
        self.target_info().scale_factor
    }
}
