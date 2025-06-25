//! Framework for positioning of popups, tooltips, and other floating UI elements.

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component, entity::Entity, query::Without, schedule::IntoScheduleConfigs,
    system::Query,
};
use bevy_math::{Rect, Vec2};
use bevy_ui::{
    ComputedNode, ComputedNodeTarget, Node, PositionType, UiGlobalTransform, UiSystems, Val,
};

/// Which side of the anchor element the floating element should be placed.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum FloatSide {
    /// The floating element should be placed above the anchor.
    Top,
    /// The floating element should be placed below the anchor.
    #[default]
    Bottom,
    /// The floating element should be placed to the left of the anchor.
    Left,
    /// The floating element should be placed to the right of the anchor.
    Right,
}

impl FloatSide {
    /// Returns the side that is the mirror image of this side.
    pub fn mirror(&self) -> Self {
        match self {
            FloatSide::Top => FloatSide::Bottom,
            FloatSide::Bottom => FloatSide::Top,
            FloatSide::Left => FloatSide::Right,
            FloatSide::Right => FloatSide::Left,
        }
    }
}

/// How the floating element should be aligned to the anchor element. The alignment will be along an
/// axis that is perpendicular to the direction of the float side. So for example, if the popup is
/// positioned below the anchor, then the [`FloatAlign`] variant controls the horizontal aligment of
/// the popup.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum FloatAlign {
    /// The starting edge of the floating element should be aligned to the starting edge of the
    /// anchor.
    #[default]
    Start,
    /// The ending edge of the floating element should be aligned to the ending edge of the anchor.
    End,
    /// The center of the floating element should be aligned to the center of the anchor.
    Center,
}

/// Indicates a possible position of a floating element relative to an anchor element. You can
/// specify multiple possible positions; the positioning code will check to see if there is
/// sufficient space to display the popup without clipping. If any position has sufficient room,
/// it will pick the first one; if there are none, then it will pick the least bad one.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FloatPosition {
    /// The side of the anchor the floating element should be placed.
    pub side: FloatSide,

    /// How the floating element should be aligned to the anchor.
    pub align: FloatAlign,

    /// If true, the floating element will be at least as large as the anchor on the adjacent
    /// side.
    pub stretch: bool,

    /// The size of the gap between the anchor and the floating element. This will offset the
    /// float along the direction of the [`FloatSide`].
    pub gap: f32,
}

/// Defines the anchor position which the floating element is positioned relative to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatAnchor {
    /// The anchor is an entity with a UI [`Node`] component.
    Node(Entity),
    /// The anchor is an arbitrary rectangle in window coordinates.
    Rect(Rect),
}

/// Component which is inserted into a floating element to make it dynamically position relative to
/// an anchor element.
#[derive(Component, PartialEq)]
pub struct Floating {
    /// The entity that this floating element is anchored to.
    pub anchor: FloatAnchor,

    /// List of potential positions for the floating element relative to the anchor.
    pub positions: Vec<FloatPosition>,
}

impl Clone for Floating {
    fn clone(&self) -> Self {
        Self {
            anchor: self.anchor,
            positions: self.positions.clone(),
        }
    }
}

fn position_floating(
    mut q_float: Query<(&mut Node, &ComputedNode, &ComputedNodeTarget, &Floating)>,
    q_anchor: Query<(&ComputedNode, &UiGlobalTransform), Without<Floating>>,
) {
    for (mut node, computed_node, computed_target, floating) in q_float.iter_mut() {
        // Logical size isn't set initially, ignore until it is.
        if computed_target.logical_size().length_squared() == 0.0 {
            continue;
        }

        // A rectangle which represents the area of the window.
        let window_rect = Rect {
            min: Vec2::ZERO,
            max: computed_target.logical_size(),
        };

        // Compute the anchor rectangle.
        let anchor_rect: Rect = match floating.anchor {
            FloatAnchor::Node(anchor_entity) => {
                let Ok((anchor_node, anchor_transform)) = q_anchor.get(anchor_entity) else {
                    continue;
                };
                Rect::from_center_size(
                    anchor_transform.translation * anchor_node.inverse_scale_factor,
                    anchor_node.size() * anchor_node.inverse_scale_factor,
                )
            }
            FloatAnchor::Rect(rect) => rect,
        };

        let mut best_occluded = f32::MAX;
        let mut best_rect = Rect::default();
        let mut best_position: FloatPosition = Default::default();

        // Loop through all the potential positions and find a good one.
        for position in &floating.positions {
            let float_size = computed_node.size() * computed_node.inverse_scale_factor;
            let mut rect = Rect::default();

            // Taraget width and height depends on whether 'stretch' is true.
            let target_width = if position.stretch && position.side == FloatSide::Top
                || position.side == FloatSide::Bottom
            {
                float_size.x.max(anchor_rect.width())
            } else {
                float_size.x
            };

            let target_height = if position.stretch && position.side == FloatSide::Left
                || position.side == FloatSide::Right
            {
                float_size.y.max(anchor_rect.height())
            } else {
                float_size.y
            };

            // Position along main axis.
            match position.side {
                FloatSide::Top => {
                    rect.max.y = anchor_rect.min.y - position.gap;
                    rect.min.y = rect.max.y - float_size.y;
                }

                FloatSide::Bottom => {
                    rect.min.y = anchor_rect.max.y + position.gap;
                    rect.max.y = rect.min.y + float_size.y;
                }

                FloatSide::Left => {
                    rect.max.x = anchor_rect.min.x - position.gap;
                    rect.min.x = rect.max.x - float_size.x;
                }

                FloatSide::Right => {
                    rect.min.x = anchor_rect.max.x + position.gap;
                    rect.max.x = rect.min.x + float_size.x;
                }
            }

            // Position along secondary axis.
            match position.align {
                FloatAlign::Start => match position.side {
                    FloatSide::Top | FloatSide::Bottom => {
                        rect.min.x = anchor_rect.min.x;
                        rect.max.x = rect.min.x + target_width;
                    }

                    FloatSide::Left | FloatSide::Right => {
                        rect.min.y = anchor_rect.min.y;
                        rect.max.y = rect.min.y + target_height;
                    }
                },

                FloatAlign::End => match position.side {
                    FloatSide::Top | FloatSide::Bottom => {
                        rect.max.x = anchor_rect.max.x;
                        rect.min.x = rect.max.x - target_width;
                    }

                    FloatSide::Left | FloatSide::Right => {
                        rect.max.y = anchor_rect.max.y;
                        rect.min.y = rect.max.y - target_height;
                    }
                },

                FloatAlign::Center => match position.side {
                    FloatSide::Top | FloatSide::Bottom => {
                        rect.min.x = (anchor_rect.width() - target_width) * 0.5;
                        rect.max.x = rect.min.x + target_width;
                    }

                    FloatSide::Left | FloatSide::Right => {
                        rect.min.y = (anchor_rect.width() - target_height) * 0.5;
                        rect.max.y = rect.min.y + target_height;
                    }
                },
            }

            // Clip to window and see how much of the floating element is occluded. We can calculate
            // how much was clipped by intersecting the rectangle against the window bounds, and
            // then subtracting the area from the area of the unclipped rectangle.
            let clipped_rect = rect.intersect(window_rect);
            let occlusion =
                rect.width() * rect.height() - clipped_rect.width() * clipped_rect.height();

            // Find the position that has the least occlusion.
            if occlusion < best_occluded {
                best_occluded = occlusion;
                best_rect = rect;
                best_position = *position;
            }
        }

        if best_occluded < f32::MAX {
            node.left = Val::Px(best_rect.min.x);
            node.top = Val::Px(best_rect.min.y);
            node.position_type = PositionType::Absolute;
            if best_position.stretch {
                match best_position.side {
                    FloatSide::Top | FloatSide::Bottom => {
                        node.min_width = Val::Px(best_rect.width());
                    }

                    FloatSide::Left | FloatSide::Right => {
                        node.min_height = Val::Px(best_rect.height());
                    }
                }
            }
        }
    }
}

/// Plugin that adds systems for the [`Floating`] component.
pub struct FloatingPlugin;

impl Plugin for FloatingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, position_floating.in_set(UiSystems::Prepare));
    }
}
