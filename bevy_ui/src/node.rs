use super::{Anchors, Margins};
use bevy_render::Color;
use glam::{self, Vec2};

#[derive(Debug, Clone)]
enum GrowDirection {
    Negative,
    Positive,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub position: Vec2,
    pub global_position: Vec2,
    pub size: Vec2,
    pub parent_dimensions: Vec2,
    pub anchors: Anchors,
    pub margins: Margins,
    pub color: Color,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            position: Vec2::default(),
            global_position: Vec2::default(),
            size: Vec2::default(),
            parent_dimensions: Vec2::default(),
            anchors: Anchors::default(),
            margins: Margins::default(),
            color: Color::rgb(0.0, 0.0, 0.0),
        }
    }
}

impl Node {
    pub fn new(position: Vec2, anchors: Anchors, margins: Margins, color: Color) -> Self {
        Node {
            position,
            global_position: Vec2::default(),
            size: Vec2::default(),
            parent_dimensions: Vec2::default(),
            anchors,
            margins,
            color,
        }
    }

    pub fn update(&mut self, parent_dimensions: Vec2, parent_position: Vec2) {
        let (rect_x, rect_width) = Self::compute_dimension_properties(
            self.position.x(),
            self.margins.left,
            self.margins.right,
            self.anchors.left,
            self.anchors.right,
            parent_dimensions.x(),
        );
        let (rect_y, rect_height) = Self::compute_dimension_properties(
            self.position.y(),
            self.margins.bottom,
            self.margins.top,
            self.anchors.bottom,
            self.anchors.top,
            parent_dimensions.y(),
        );

        self.size = glam::vec2(rect_width, rect_height);
        self.global_position = glam::vec2(rect_x, rect_y) + parent_position;
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
            GrowDirection::Positive
        } else {
            GrowDirection::Negative
        };
        let p1_grow_direction = if anchor_p1 < 0.5 {
            GrowDirection::Positive
        } else {
            GrowDirection::Negative
        };

        let p0 = Self::compute_rect_position(offset, margin0, anchor_p0, p0_grow_direction);
        let p1 = Self::compute_rect_position(offset, margin1, anchor_p1, p1_grow_direction);

        let final_width = p1 - p0;
        let mut p = (p0 + p1) / 2.0;

        // move position to "origin" in bottom left hand corner
        p = p - final_width / 2.0;

        (p, final_width)
    }

    fn compute_rect_position(
        position: f32,
        margin: f32,
        anchor_position: f32,
        grow_direction: GrowDirection,
    ) -> f32 {
        match grow_direction {
            GrowDirection::Negative => position + anchor_position - margin,
            GrowDirection::Positive => position + anchor_position + margin,
        }
    }
}
