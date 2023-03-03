use crate::BorderColor;
use crate::Node;
use crate::Style;
use crate::Val;
use bevy_ecs::prelude::Bundle;
use bevy_ecs::prelude::Component;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Or;
use bevy_ecs::query::With;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::system::Query;
use bevy_hierarchy::Children;
use bevy_hierarchy::Parent;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_reflect::Reflect;

/// Stores the calculated border geometry
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedBorder {
    /// The four rects that make up the border
    pub edges: [Option<Rect>; 4],
}

impl CalculatedBorder {
    const DEFAULT: Self = Self { edges: [None; 4] };
}

impl Default for CalculatedBorder {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A bundle containing the components needed for a Node to display a border
#[derive(Bundle, Default, Clone)]
pub struct BorderBundle {
    pub border_color: BorderColor,
    pub calculated_border: CalculatedBorder,
}

/// Percentage thickness of all border edges is calculated based on the width of the parent node.
fn resolve_thickness(value: Val, parent_width: f32) -> f32 {
    match value {
        Val::Auto | Val::Undefined => 0.,
        Val::Px(px) => px,
        Val::Percent(percent) => parent_width * percent / 100.,
    }
}

/// Generates the border geometry
pub fn calculate_borders_system(
    parent_query: Query<&Node, With<Children>>,
    mut border_query: Query<
        (&Node, &Style, &mut CalculatedBorder, Option<&Parent>),
        Or<(Changed<Node>, Changed<Style>, Changed<Parent>)>,
    >,
) {
    for (node, style, mut calculated_border, parent) in border_query.iter_mut() {
        let node_size = node.calculated_size;
        if node_size.x <= 0. || node_size.y <= 0. {
            calculated_border.edges = [None; 4];
            continue;
        }

        let parent_width = parent
            .and_then(|parent| parent_query.get(parent.get()).ok())
            .map(|parent_node| parent_node.calculated_size.x)
            .unwrap_or(0.);
        let border = style.border;
        let left = resolve_thickness(border.left, parent_width);
        let right = resolve_thickness(border.right, parent_width);
        let top = resolve_thickness(border.top, parent_width);
        let bottom = resolve_thickness(border.bottom, parent_width);

        // calculate border rects, ensuring that they don't overlap
        let max = 0.5 * node_size;
        let min = -max;
        let inner_min = min + Vec2::new(left, top);
        let inner_max = (max - Vec2::new(right, bottom)).max(inner_min);

        let border_rects = [
            // Left border
            Rect {
                min,
                max: Vec2::new(inner_min.x, max.y),
            },
            // Right border
            Rect {
                min: Vec2::new(inner_max.x, min.y),
                max,
            },
            // Top border
            Rect {
                min: Vec2::new(inner_min.x, min.y),
                max: Vec2::new(inner_max.x, inner_min.y),
            },
            // Bottom border
            Rect {
                min: Vec2::new(inner_min.x, inner_max.y),
                max: Vec2::new(inner_max.x, max.y),
            },
        ];

        for (i, edge) in border_rects.into_iter().enumerate() {
            calculated_border.edges[i] = if edge.min.x < edge.max.x && edge.min.y < edge.max.y {
                Some(edge)
            } else {
                None
            };
        }
    }
}
