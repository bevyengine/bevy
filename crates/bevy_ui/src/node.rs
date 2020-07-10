use super::{Anchors, Margins};
use bevy_render::render_resource::RenderResources;
use bevy_transform::prelude::Translation;
use glam::{Vec2, Vec3};

#[derive(Debug, Clone)]
enum MarginGrowDirection {
    Negative,
    Positive,
}

#[derive(Debug, Clone, Default, RenderResources)]
pub struct Node {
    pub size: Vec2,
    #[render_resources(ignore)]
    pub position: Vec2,
    #[render_resources(ignore)]
    pub anchors: Anchors,
    #[render_resources(ignore)]
    pub margins: Margins,
}

impl Node {
    pub fn new(anchors: Anchors, margins: Margins) -> Self {
        Node {
            anchors,
            margins,
            ..Default::default()
        }
    }

    pub fn positioned(position: Vec2, anchors: Anchors, margins: Margins) -> Self {
        Node {
            position,
            anchors,
            margins,
            ..Default::default()
        }
    }

    pub fn update(&mut self, translation: &mut Translation, z_offset: f32, parent_size: Vec2) {
        let (quad_x, quad_width) = Self::compute_dimension_properties(
            self.margins.left,
            self.margins.right,
            self.anchors.left,
            self.anchors.right,
            parent_size.x(),
        );
        let (quad_y, quad_height) = Self::compute_dimension_properties(
            self.margins.bottom,
            self.margins.top,
            self.anchors.bottom,
            self.anchors.top,
            parent_size.y(),
        );

        self.size = Vec2::new(quad_width, quad_height);
        translation.0 = self.position.extend(0.0) + Vec3::new(quad_x, quad_y, z_offset)
            - (parent_size / 2.0).extend(0.0);
    }

    fn compute_dimension_properties(
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

        let p0 = Self::compute_anchored_position(margin0, anchor_p0, p0_grow_direction);
        let p1 = Self::compute_anchored_position(margin1, anchor_p1, p1_grow_direction);

        let final_width = p1 - p0;
        let p = (p0 + p1) / 2.0;
        (p, final_width.abs())
    }

    fn compute_anchored_position(
        margin: f32,
        anchor_position: f32,
        grow_direction: MarginGrowDirection,
    ) -> f32 {
        match grow_direction {
            MarginGrowDirection::Negative => anchor_position - margin,
            MarginGrowDirection::Positive => anchor_position + margin,
        }
    }
}
