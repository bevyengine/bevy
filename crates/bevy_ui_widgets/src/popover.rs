//! Framework for positioning of popups, tooltips, and other popover UI elements.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChangesMut, component::Component, hierarchy::ChildOf, query::Without,
    schedule::IntoScheduleConfigs, system::Query,
};
use bevy_math::{Rect, Vec2};
use bevy_ui::{
    ComputedNode, ComputedUiRenderTargetInfo, Node, PositionType, UiGlobalTransform, UiSystems, Val,
};

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

fn position_popover(
    mut q_popover: Query<(
        &mut Node,
        &mut Visibility,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &Popover,
        &ChildOf,
    )>,
    q_parent: Query<(&ComputedNode, &UiGlobalTransform), Without<Popover>>,
) {
    for (mut node, mut visibility, computed_node, computed_target, popover, parent) in
        q_popover.iter_mut()
    {
        // A rectangle which represents the area of the window.
        let window_rect = Rect {
            min: Vec2::ZERO,
            max: computed_target.logical_size(),
        }
        .inflate(-popover.window_margin);

        // Compute the parent rectangle.
        let Ok((parent_node, parent_transform)) = q_parent.get(parent.parent()) else {
            continue;
        };
        // Computed node size includes the border, but since absolute positioning doesn't include
        // border we need to remove it from the calculations.
        let parent_size = parent_node.size()
            - Vec2::new(
                parent_node.border.left + parent_node.border.right,
                parent_node.border.top + parent_node.border.bottom,
            );
        let parent_rect = scale_rect(
            Rect::from_center_size(parent_transform.translation, parent_size),
            parent_node.inverse_scale_factor,
        );

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
            let left = Val::Px(best_rect.min.x - parent_rect.min.x);
            let top = Val::Px(best_rect.min.y - parent_rect.min.y);
            visibility.set_if_neq(Visibility::Visible);
            if node.left != left {
                node.left = left;
            }
            if node.top != top {
                node.top = top;
            }
            if node.bottom != Val::DEFAULT {
                node.bottom = Val::DEFAULT;
            }
            if node.right != Val::DEFAULT {
                node.right = Val::DEFAULT;
            }
            if node.position_type != PositionType::Absolute {
                node.position_type = PositionType::Absolute;
            }
        }
    }
}

/// Plugin that adds systems for the [`Popover`] component.
pub struct PopoverPlugin;

impl Plugin for PopoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, position_popover.in_set(UiSystems::Prepare));
    }
}

#[inline]
fn scale_rect(rect: Rect, factor: f32) -> Rect {
    Rect {
        min: rect.min * factor,
        max: rect.max * factor,
    }
}
