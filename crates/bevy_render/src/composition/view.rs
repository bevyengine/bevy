use bevy_ecs::{
    component::{Component, HookContext},
    entity::Entity,
    query::With,
    system::{Commands, Query, Single},
    world::DeferredWorld,
};
use bevy_math::{Rect, URect, UVec2, UVec4, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::GlobalTransform;
use bevy_window::PrimaryWindow;
use tracing::warn;

use core::ops::Range;
use std::sync::Arc;

use crate::{
    primitives::SubRect,
    render_graph::InternedRenderSubGraph,
    sync_world::{RenderEntity, SyncToRenderWorld},
    Extract,
};

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
    pub fn from_sub_rect(sub_rect: SubRect, physicsl_size: UVec2) -> Self {
        let scaled = sub_rect.scaled_roughly_to(physicsl_size);
        Self {
            physical_position: scaled.offset.as_uvec2(),
            physical_size: scaled.size,
            ..Default::default()
        }
    }

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
#[component(immutable)] // Note: immutable not for internal use, but for other things like
                        // projection to watch with observers.
pub struct ViewTarget {
    pub(super) target: Arc<(NormalizedRenderTarget, RenderTargetInfo)>,
    pub(super) viewport: Option<Viewport>,
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

// -----------------------------------------------------------------------------
// Extraction / Render World Logic

/// Describes a camera in the render world.
///
/// Each entity in the main world can potentially extract to multiple subviews,
/// each of which has a [`RetainedViewEntity::subview_index`]. For instance, 3D
/// cameras extract to both a 3D camera subview with index 0 and a special UI
/// subview with index 1. Likewise, point lights with shadows extract to 6
/// subviews, one for each side of the shadow cubemap.
#[derive(Component)]
pub struct ExtractedView {
    /// The entity in the main world corresponding to this render world view.
    pub retained_view_entity: RetainedViewEntity,
    pub render_graph: InternedRenderSubGraph,
    /// The render target entity associated with this View
    pub target: NormalizedRenderTarget,
    pub physical_viewport_size: Option<UVec2>,
    pub physical_target_size: Option<UVec2>,
    // uvec4(origin.x, origin.y, width, height)
    pub viewport: Option<UVec4>,
}

pub fn extract_views(
    mut commands: Commands,
    views: Extract<
        Query<(
            Entity,
            RenderEntity,
            &View,
            Option<&ViewTarget>,
            &RenderGraphDriver,
        )>,
    >,
    primary_window: Extract<Option<Single<Entity, With<PrimaryWindow>>>>,
    mapper: Extract<Query<&RenderEntity>>,
) {
    for (entity, render_entity, view, view_target, view_render_graph) in &views {
        let extracted_view = view_target.map(|view_target| ExtractedView {
            retained_view_entity: RetainedViewEntity::new(main_entity.into(), None, 0),
            render_graph: *view_render_graph,
            target: view_target.target.0.clone(),
            physical_viewport_size: ,
            physical_target_size: todo!(),
            viewport: todo!(),
        });
    }
    // let primary_window = primary_window.iter().next();
    // for (
    //     main_entity,
    //     render_entity,
    //     view,
    //     camera_render_graph,
    //     camera,
    //     transform,
    //     visible_entities,
    //     frustum,
    //     hdr,
    //     color_grading,
    //     exposure,
    //     temporal_jitter,
    //     render_layers,
    //     projection,
    //     no_indirect_drawing,
    // ) in query.iter()
    // {
    //     if !camera.is_active {
    //         commands.entity(render_entity).remove::<(
    //             ExtractedCamera,
    //             ExtractedView,
    //             RenderVisibleEntities,
    //             TemporalJitter,
    //             RenderLayers,
    //             Projection,
    //             NoIndirectDrawing,
    //             ViewUniformOffset,
    //         )>();
    //         continue;
    //     }
    //
    //     let color_grading = color_grading.unwrap_or(&ColorGrading::default()).clone();
    //
    //     if let (
    //         Some(URect {
    //             min: viewport_origin,
    //             ..
    //         }),
    //         Some(viewport_size),
    //         Some(target_size),
    //     ) = (
    //         camera.physical_viewport_rect(),
    //         camera.physical_viewport_size(),
    //         camera.physical_target_size(),
    //     ) {
    //         if target_size.x == 0 || target_size.y == 0 {
    //             continue;
    //         }
    //
    //         let render_visible_entities = RenderVisibleEntities {
    //             entities: visible_entities
    //                 .entities
    //                 .iter()
    //                 .map(|(type_id, entities)| {
    //                     let entities = entities
    //                         .iter()
    //                         .map(|entity| {
    //                             let render_entity = mapper
    //                                 .get(*entity)
    //                                 .cloned()
    //                                 .map(|entity| entity.id())
    //                                 .unwrap_or(Entity::PLACEHOLDER);
    //                             (render_entity, (*entity).into())
    //                         })
    //                         .collect();
    //                     (*type_id, entities)
    //                 })
    //                 .collect(),
    //         };
    //
    //         let mut commands = commands.entity(render_entity);
    //         commands.insert((
    //             ExtractedCamera {
    //                 target: camera.target.normalize(primary_window),
    //                 viewport: camera.viewport.clone(),
    //                 physical_viewport_size: Some(viewport_size),
    //                 physical_target_size: Some(target_size),
    //                 render_graph: camera_render_graph.0,
    //                 order: camera.order,
    //                 output_mode: camera.output_mode,
    //                 msaa_writeback: camera.msaa_writeback,
    //                 clear_color: camera.clear_color,
    //                 // this will be set in sort_cameras
    //                 sorted_camera_index_for_target: 0,
    //                 exposure: exposure
    //                     .map(Exposure::exposure)
    //                     .unwrap_or_else(|| Exposure::default().exposure()),
    //                 hdr,
    //             },
    //             ExtractedView {
    //                 retained_view_entity: RetainedViewEntity::new(main_entity.into(), None, 0),
    //                 clip_from_view: camera.clip_from_view(),
    //                 world_from_view: *transform,
    //                 clip_from_world: None,
    //                 hdr,
    //                 viewport: UVec4::new(
    //                     viewport_origin.x,
    //                     viewport_origin.y,
    //                     viewport_size.x,
    //                     viewport_size.y,
    //                 ),
    //                 color_grading,
    //             },
    //             render_visible_entities,
    //             *frustum,
    //         ));
    //
    //         if let Some(temporal_jitter) = temporal_jitter {
    //             commands.insert(temporal_jitter.clone());
    //         }
    //
    //         if let Some(render_layers) = render_layers {
    //             commands.insert(render_layers.clone());
    //         }
    //
    //         if let Some(perspective) = projection {
    //             commands.insert(perspective.clone());
    //         }
    //
    //         if no_indirect_drawing
    //             || !matches!(
    //                 gpu_preprocessing_support.max_supported_mode,
    //                 GpuPreprocessingMode::Culling
    //             )
    //         {
    //             commands.insert(NoIndirectDrawing);
    //         }
    //     };
    // }
}
