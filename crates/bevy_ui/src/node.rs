use super::{Anchors, Margins};
use bevy_sprite::Rect;
use glam::Vec2;

#[derive(Debug, Clone)]
enum MarginGrowDirection {
    Negative,
    Positive,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub position: Vec2,
    pub anchors: Anchors,
    pub margins: Margins,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            position: Vec2::default(),
            anchors: Anchors::default(),
            margins: Margins::default(),
        }
    }
}

impl Node {
    pub fn new(position: Vec2, anchors: Anchors, margins: Margins) -> Self {
        Node {
            position,
            anchors,
            margins,
        }
    }

    pub fn update(
        &mut self,
        rect: &mut Rect,
        parent_size: Vec2,
        parent_position: Vec2,
        z_index: f32,
    ) {
        let (rect_x, rect_width) = Self::compute_dimension_properties(
            self.position.x(),
            self.margins.left,
            self.margins.right,
            self.anchors.left,
            self.anchors.right,
            parent_size.x(),
        );
        let (rect_y, rect_height) = Self::compute_dimension_properties(
            self.position.y(),
            self.margins.bottom,
            self.margins.top,
            self.anchors.bottom,
            self.anchors.top,
            parent_size.y(),
        );

        rect.size = Vec2::new(rect_width, rect_height);
        rect.position = Vec2::new(rect_x, rect_y) + parent_position;
        rect.z_index = z_index;
    }

    fn compute_dimension_properties(
        offset: f32,
        margin0: f32,
        margin1: f32,
        anchor0: f32,
        anchor1: f32,
        length: f32,
    ) -> (f32, f32) {
        let anchor_p0 = anchor0 * length;
        let anchor_p1 = anchor1 * length;

        let p0_grow_direction = if anchor_p0 <= 0.5 {
            MarginGrowDirection::Positive
        } else {
            MarginGrowDirection::Negative
        };
        let p1_grow_direction = if anchor_p1 <= 0.5 {
            MarginGrowDirection::Positive
        } else {
            MarginGrowDirection::Negative
        };

        let p0 = Self::compute_rect_position(offset, margin0, anchor_p0, p0_grow_direction);
        let p1 = Self::compute_rect_position(offset, margin1, anchor_p1, p1_grow_direction);

        let final_width = p1 - p0;
        let p = (p0 + p1) / 2.0;
        (p, final_width.abs())
    }

    fn compute_rect_position(
        position: f32,
        margin: f32,
        anchor_position: f32,
        grow_direction: MarginGrowDirection,
    ) -> f32 {
        match grow_direction {
            MarginGrowDirection::Negative => position + anchor_position - margin,
            MarginGrowDirection::Positive => position + anchor_position + margin,
        }
    }
}
