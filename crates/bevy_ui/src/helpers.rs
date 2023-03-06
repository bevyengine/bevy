use bevy_math::Rect;
use bevy_transform::prelude::GlobalTransform;

use crate::Node;

// calculate the geometry of a ui node in logical pixels
pub fn logical_node_rect(node: &Node, transform: &GlobalTransform) -> Rect {
    Rect::from_center_size(transform.translation().truncate(), node.size())
}

// calculate the geometry of a ui node in physical pixels
pub fn physical_node_rect(node: &Node, transform: &GlobalTransform, scale_factor: f32) -> Rect {
    let rect = logical_node_rect(node, transform);
    Rect {
        min: rect.min / scale_factor,
        max: rect.max / scale_factor,
    }
}