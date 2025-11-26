use crate::{
    experimental::{UiChildren, UiRootNodes},
    ui_transform::{UiGlobalTransform, UiTransform},
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, Display, IgnoreScroll, LayoutConfig,
    Node, Outline, OverflowAxis, ScrollPosition,
};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::Added,
    system::{Query, ResMut},
    world::Ref,
};

use bevy_math::{Affine2, Vec2};
use bevy_sprite::BorderRect;
use thiserror::Error;
use ui_surface::UiSurface;

use bevy_text::ComputedTextLayout;

mod convert;
pub mod debug;
pub(crate) mod ui_surface;

pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
}

impl LayoutContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
    };
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    #[inline]
    const fn new(scale_factor: f32, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
        }
    }
}

#[cfg(test)]
impl LayoutContext {
    pub const TEST_CONTEXT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::new(1000.0, 1000.0),
    };
}

impl Default for LayoutContext {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Invalid hierarchy")]
    InvalidHierarchy,
    #[error("Taffy error: {0}")]
    TaffyError(taffy::tree::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    mut ui_surface: ResMut<UiSurface>,
    ui_root_node_query: UiRootNodes,
    ui_children: UiChildren,
    mut node_query: Query<(
        Entity,
        Ref<Node>,
        Option<&mut ContentSize>,
        Ref<ComputedUiRenderTargetInfo>,
    )>,
    added_node_query: Query<(), Added<Node>>,
    mut node_update_query: Query<(
        &mut ComputedNode,
        &UiTransform,
        &mut UiGlobalTransform,
        &Node,
        Option<&LayoutConfig>,
        Option<&Outline>,
        Option<&ScrollPosition>,
        Option<&IgnoreScroll>,
    )>,
    mut buffer_query: Query<&mut ComputedTextLayout>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_nodes: RemovedComponents<Node>,
) {
    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.read() {
        ui_surface.try_remove_node_context(entity);
    }

    // Sync Node and ContentSize to Taffy for all nodes
    node_query
        .iter_mut()
        .for_each(|(entity, node, content_size, computed_target)| {
            if computed_target.is_changed()
                || node.is_changed()
                || content_size
                    .as_ref()
                    .is_some_and(|c| c.is_changed() || c.measure.is_some())
            {
                let layout_context = LayoutContext::new(
                    computed_target.scale_factor,
                    computed_target.physical_size.as_vec2(),
                );
                let measure = content_size.and_then(|mut c| c.measure.take());
                ui_surface.upsert_node(&layout_context, entity, &node, measure);
            }
        });

    // update and remove children
    for entity in removed_children.read() {
        ui_surface.try_remove_children(entity);
    }

    // clean up removed nodes after syncing children to avoid potential panic (invalid SlotMap key used)
    ui_surface.remove_entities(
        removed_nodes
            .read()
            .filter(|entity| !node_query.contains(*entity)),
    );

    for ui_root_entity in ui_root_node_query.iter() {
        fn update_children_recursively(
            ui_surface: &mut UiSurface,
            ui_children: &UiChildren,
            added_node_query: &Query<(), Added<Node>>,
            entity: Entity,
        ) {
            if ui_surface.entity_to_taffy.contains_key(&entity)
                && (added_node_query.contains(entity)
                    || ui_children.is_changed(entity)
                    || ui_children
                        .iter_ui_children(entity)
                        .any(|child| added_node_query.contains(child)))
            {
                ui_surface.update_children(entity, ui_children.iter_ui_children(entity));
            }

            for child in ui_children.iter_ui_children(entity) {
                update_children_recursively(ui_surface, ui_children, added_node_query, child);
            }
        }

        update_children_recursively(
            &mut ui_surface,
            &ui_children,
            &added_node_query,
            ui_root_entity,
        );

        let (_, _, _, computed_target) = node_query.get(ui_root_entity).unwrap();

        ui_surface.compute_layout(
            ui_root_entity,
            computed_target.physical_size,
            &mut buffer_query,
        );

        update_uinode_geometry_recursive(
            ui_root_entity,
            &mut ui_surface,
            true,
            computed_target.physical_size().as_vec2(),
            Affine2::IDENTITY,
            &mut node_update_query,
            &ui_children,
            computed_target.scale_factor.recip(),
            Vec2::ZERO,
            Vec2::ZERO,
        );
    }

    // Returns the combined bounding box of the node and any of its overflowing children.
    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &mut UiSurface,
        inherited_use_rounding: bool,
        target_size: Vec2,
        mut inherited_transform: Affine2,
        node_update_query: &mut Query<(
            &mut ComputedNode,
            &UiTransform,
            &mut UiGlobalTransform,
            &Node,
            Option<&LayoutConfig>,
            Option<&Outline>,
            Option<&ScrollPosition>,
            Option<&IgnoreScroll>,
        )>,
        ui_children: &UiChildren,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        parent_scroll_position: Vec2,
    ) {
        if let Ok((
            mut node,
            transform,
            mut global_transform,
            style,
            maybe_layout_config,
            maybe_outline,
            maybe_scroll_position,
            maybe_scroll_sticky,
        )) = node_update_query.get_mut(entity)
        {
            let use_rounding = maybe_layout_config
                .map(|layout_config| layout_config.use_rounding)
                .unwrap_or(inherited_use_rounding);

            let Ok((layout, unrounded_size)) = ui_surface.get_layout(entity, use_rounding) else {
                return;
            };

            let layout_size = Vec2::new(layout.size.width, layout.size.height);

            // Taffy layout position of the top-left corner of the node, relative to its parent.
            let layout_location = Vec2::new(layout.location.x, layout.location.y);

            // If IgnoreScroll is set, parent scroll position is ignored along the specified axes.
            let effective_parent_scroll = maybe_scroll_sticky
                .map(|scroll_sticky| parent_scroll_position * Vec2::from(!scroll_sticky.0))
                .unwrap_or(parent_scroll_position);

            // The position of the center of the node relative to its top-left corner.
            let local_center =
                layout_location - effective_parent_scroll + 0.5 * (layout_size - parent_size);

            // only trigger change detection when the new values are different
            if node.size != layout_size
                || node.unrounded_size != unrounded_size
                || node.inverse_scale_factor != inverse_target_scale_factor
            {
                node.size = layout_size;
                node.unrounded_size = unrounded_size;
                node.inverse_scale_factor = inverse_target_scale_factor;
            }

            let content_size = Vec2::new(layout.content_size.width, layout.content_size.height);
            node.bypass_change_detection().content_size = content_size;

            let taffy_rect_to_border_rect = |rect: taffy::Rect<f32>| BorderRect {
                left: rect.left,
                right: rect.right,
                top: rect.top,
                bottom: rect.bottom,
            };

            node.bypass_change_detection().border = taffy_rect_to_border_rect(layout.border);
            node.bypass_change_detection().padding = taffy_rect_to_border_rect(layout.padding);

            // Compute the node's new global transform
            let mut local_transform = transform.compute_affine(
                inverse_target_scale_factor.recip(),
                layout_size,
                target_size,
            );
            local_transform.translation += local_center;
            inherited_transform *= local_transform;

            if inherited_transform != **global_transform {
                *global_transform = inherited_transform.into();
            }

            // We don't trigger change detection for changes to border radius
            node.bypass_change_detection().border_radius = style.border_radius.resolve(
                inverse_target_scale_factor.recip(),
                node.size,
                target_size,
            );

            if let Some(outline) = maybe_outline {
                // don't trigger change detection when only outlines are changed
                let node = node.bypass_change_detection();
                node.outline_width = if style.display != Display::None {
                    outline
                        .width
                        .resolve(
                            inverse_target_scale_factor.recip(),
                            node.size().x,
                            target_size,
                        )
                        .unwrap_or(0.)
                        .max(0.)
                } else {
                    0.
                };

                node.outline_offset = outline
                    .offset
                    .resolve(
                        inverse_target_scale_factor.recip(),
                        node.size().x,
                        target_size,
                    )
                    .unwrap_or(0.)
                    .max(0.);
            }

            node.bypass_change_detection().scrollbar_size =
                Vec2::new(layout.scrollbar_size.width, layout.scrollbar_size.height);

            let scroll_position: Vec2 = maybe_scroll_position
                .map(|scroll_pos| {
                    Vec2::new(
                        if style.overflow.x == OverflowAxis::Scroll {
                            scroll_pos.x * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                        if style.overflow.y == OverflowAxis::Scroll {
                            scroll_pos.y * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                    )
                })
                .unwrap_or_default();

            let max_possible_offset =
                (content_size - layout_size + node.scrollbar_size).max(Vec2::ZERO);
            let clamped_scroll_position = scroll_position.clamp(Vec2::ZERO, max_possible_offset);

            let physical_scroll_position = clamped_scroll_position.floor();

            node.bypass_change_detection().scroll_position = physical_scroll_position;

            for child_uinode in ui_children.iter_ui_children(entity) {
                update_uinode_geometry_recursive(
                    child_uinode,
                    ui_surface,
                    use_rounding,
                    target_size,
                    inherited_transform,
                    node_update_query,
                    ui_children,
                    inverse_target_scale_factor,
                    layout_size,
                    physical_scroll_position,
                );
            }
        }
    }
}
