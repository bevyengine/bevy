//! Framework for positioning of popups, tooltips, and other popover UI elements.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::Without,
    schedule::IntoScheduleConfigs,
    system::{ParamSet, Query},
};
use bevy_math::{Affine2, Rect, Vec2};
use bevy_ui::{
    ui_layout_system, ComputedNode, ComputedUiRenderTargetInfo, Node, PositionType,
    UiGlobalTransform, UiSystems, UiTransform, Val2,
};

use crate::update_scrollbar_thumb;

/// Which side of the parent element the popover element should be placed.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum PopoverSide {
    /// The popover element should be placed above the parent.
    Top,
    /// The popover element should be placed below the parent.
    #[default]
    Bottom,
    /// The popover element should be placed to the left of the parent.
    Left,
    /// The popover element should be placed to the right of the parent.
    Right,
}

impl PopoverSide {
    /// Returns the side that is the mirror image of this side.
    pub fn mirror(&self) -> Self {
        match self {
            PopoverSide::Top => PopoverSide::Bottom,
            PopoverSide::Bottom => PopoverSide::Top,
            PopoverSide::Left => PopoverSide::Right,
            PopoverSide::Right => PopoverSide::Left,
        }
    }
}

/// How the popover element should be aligned to the parent element. The alignment will be along an
/// axis that is perpendicular to the direction of the popover side. So for example, if the popup is
/// positioned below the parent, then the [`PopoverAlign`] variant controls the horizontal alignment
/// of the popup.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum PopoverAlign {
    /// The starting edge of the popover element should be aligned to the starting edge of the
    /// parent.
    #[default]
    Start,
    /// The center of the popover element should be aligned to the center of the parent.
    Center,
    /// The ending edge of the popover element should be aligned to the ending edge of the parent.
    End,
}

/// Indicates a possible position of a popover element relative to it's parent. You can
/// specify multiple possible positions; the positioning code will check to see if there is
/// sufficient space to display the popup without being clipped by the window edge. If any position
/// has sufficient room, it will pick the first one; if there are none, then it will pick the least
/// bad one.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PopoverPlacement {
    /// The side of the parent entity where the popover element should be placed.
    pub side: PopoverSide,

    /// How the popover element should be aligned to the parent entity.
    pub align: PopoverAlign,

    /// The size of the gap between the parent and the popover element, in logical pixels. This will
    /// offset the popover along the direction of `side`.
    pub gap: f32,
}

/// Component which is inserted into a popover element to make it dynamically position relative to
/// an parent element.
#[derive(Component, PartialEq, Default)]
pub struct Popover {
    /// List of potential positions for the popover element relative to the parent.
    pub positions: Vec<PopoverPlacement>,

    /// Indicates how close to the window edge the popup is allowed to go.
    pub window_margin: f32,
}

impl Clone for Popover {
    fn clone(&self) -> Self {
        Self {
            positions: self.positions.clone(),
            window_margin: self.window_margin,
        }
    }
}

pub(crate) fn position_popover(
    mut q_popover: Query<(
        Entity,
        &mut Node,
        &mut UiTransform,
        &mut UiGlobalTransform,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &Popover,
        &ChildOf,
    )>,
    mut qs_transform: ParamSet<(
        Query<(&ComputedNode, &UiGlobalTransform), Without<Popover>>,
        Query<&mut UiGlobalTransform, Without<Popover>>,
    )>,
    q_children: Query<&Children>,
) {
    for (
        popover_entity,
        mut node,
        mut transform,
        mut ui_global_transform,
        computed_node,
        computed_target,
        popover,
        parent,
    ) in q_popover.iter_mut()
    {
        // A rectangle which represents the area of the window.
        let window_rect = Rect {
            min: Vec2::ZERO,
            max: computed_target.logical_size(),
        }
        .inflate(-popover.window_margin);

        // Compute the parent rectangle.
        let q_parent = qs_transform.p0();
        let Ok((parent_node, parent_transform)) = q_parent.get(parent.parent()) else {
            continue;
        };

        // Computed node size includes the border, but since absolute positioning doesn't include
        // border we need to remove it from the calculations.
        let parent_size =
            parent_node.size() - parent_node.border.min_inset - parent_node.border.max_inset;
        let parent_rect = scale_rect(
            Rect::from_center_size(parent_transform.translation, parent_size),
            parent_node.inverse_scale_factor,
        );
        let parent_matrix = parent_transform.affine().matrix2;

        let mut best_occluded = f32::MAX;
        let mut best_rect = Rect::default();

        // Loop through all the potential positions and find a good one.
        for position in &popover.positions {
            let popover_size = computed_node.size() * computed_node.inverse_scale_factor;
            let mut rect = Rect::default();

            let target_width = popover_size.x;
            let target_height = popover_size.y;

            // Position along main axis.
            match position.side {
                PopoverSide::Top => {
                    rect.max.y = parent_rect.min.y - position.gap;
                    rect.min.y = rect.max.y - popover_size.y;
                }

                PopoverSide::Bottom => {
                    rect.min.y = parent_rect.max.y + position.gap;
                    rect.max.y = rect.min.y + popover_size.y;
                }

                PopoverSide::Left => {
                    rect.max.x = parent_rect.min.x - position.gap;
                    rect.min.x = rect.max.x - popover_size.x;
                }

                PopoverSide::Right => {
                    rect.min.x = parent_rect.max.x + position.gap;
                    rect.max.x = rect.min.x + popover_size.x;
                }
            }

            // Position along secondary axis.
            match position.align {
                PopoverAlign::Start => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.min.x = parent_rect.min.x;
                        rect.max.x = rect.min.x + target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.min.y = parent_rect.min.y;
                        rect.max.y = rect.min.y + target_height;
                    }
                },

                PopoverAlign::End => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.max.x = parent_rect.max.x;
                        rect.min.x = rect.max.x - target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.max.y = parent_rect.max.y;
                        rect.min.y = rect.max.y - target_height;
                    }
                },

                PopoverAlign::Center => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.min.x = parent_rect.min.x + (parent_rect.width() - target_width) * 0.5;
                        rect.max.x = rect.min.x + target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.min.y =
                            parent_rect.min.y + (parent_rect.height() - target_height) * 0.5;
                        rect.max.y = rect.min.y + target_height;
                    }
                },
            }

            // Clip to window and see how much of the popover element is occluded. We can calculate
            // how much was clipped by intersecting the rectangle against the window bounds, and
            // then subtracting the area from the area of the unclipped rectangle.
            let clipped_rect = rect.intersect(window_rect);
            let occlusion = rect.area() - clipped_rect.area();

            // Find the position that has the least occlusion.
            if occlusion < best_occluded {
                best_occluded = occlusion;
                best_rect = rect;
            }
        }

        // Update node properties, but only if they are different from before (to avoid setting
        // change detection bit).
        if best_occluded < f32::MAX {
            let best_center = 0.5 * (best_rect.min + best_rect.max);
            let current_center =
                ui_global_transform.translation * computed_node.inverse_scale_factor;
            let physical_translation =
                (best_center - current_center) * computed_target.scale_factor();
            if parent_matrix.determinant() == 0.0 {
                continue;
            }
            let resolved_translation = transform.translation.resolve(
                computed_target.scale_factor(),
                computed_node.size(),
                computed_target.physical_size().as_vec2(),
            );
            let logical_translation = (resolved_translation
                + parent_matrix.inverse() * physical_translation)
                / computed_target.scale_factor();
            let ui_translation = Val2::px(logical_translation.x, logical_translation.y);
            if transform.translation != ui_translation {
                transform.translation = ui_translation;
            }
            if node.position_type != PositionType::Absolute {
                node.position_type = PositionType::Absolute;
            }

            if physical_translation != Vec2::ZERO {
                let mut affine = ui_global_transform.affine();
                affine.translation += physical_translation;
                *ui_global_transform = affine.into();

                if let Ok(children) = q_children.get(popover_entity) {
                    for child in children.iter() {
                        translate_ui_children_recursive(
                            *child,
                            physical_translation,
                            &q_children,
                            &mut qs_transform.p1(),
                        );
                    }
                }
            }
        }
    }
}

fn translate_ui_children_recursive(
    entity: Entity,
    translation: Vec2,
    q_children: &Query<&Children>,
    q_transform: &mut Query<&mut UiGlobalTransform, Without<Popover>>,
) {
    let Ok(mut ui_global_transform) = q_transform.get_mut(entity) else {
        return;
    };

    *ui_global_transform =
        (ui_global_transform.affine() * Affine2::from_translation(translation)).into();

    if let Ok(children) = q_children.get(entity) {
        for child in children.iter() {
            translate_ui_children_recursive(*child, translation, q_children, q_transform);
        }
    }
}

/// Plugin that adds systems for the [`Popover`] component.
pub struct PopoverPlugin;

impl Plugin for PopoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            position_popover
                .in_set(UiSystems::Layout)
                // The Stack systems just modify the stack_index of ComputedNode, which this system
                // doesn't use.
                .ambiguous_with(UiSystems::Stack)
                .after(ui_layout_system)
                .before(update_scrollbar_thumb),
        );
    }
}

#[inline]
fn scale_rect(rect: Rect, factor: f32) -> Rect {
    Rect {
        min: rect.min * factor,
        max: rect.max * factor,
    }
}
