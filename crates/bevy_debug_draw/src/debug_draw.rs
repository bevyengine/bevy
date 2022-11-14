use std::{f32::consts::TAU, mem};

use bevy_asset::Handle;
use bevy_ecs::system::Resource;
use bevy_math::{vec3, Quat, Vec2, Vec3};
use bevy_render::prelude::{Color, Mesh};

/// A resource with an immediate mode API for drawing lines and wireshapes.
/// Useful for visual debugging.
#[derive(Resource)]
pub struct DebugDraw {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    pub(crate) mesh_handle: Option<Handle<Mesh>>,
    /// The amount of line segments to use when drawing a circle.
    ///
    /// Defaults to `24`.
    pub circle_segments: u32,
}

impl Default for DebugDraw {
    fn default() -> Self {
        DebugDraw {
            positions: Vec::new(),
            colors: Vec::new(),
            mesh_handle: None,
            circle_segments: 24,
        }
    }
}

impl DebugDraw {
    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.line_gradient(start, end, color, color);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient(&mut self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        self.positions.extend([start.to_array(), end.to_array()]);
        self.colors.extend([
            start_color.as_linear_rgba_f32(),
            end_color.as_linear_rgba_f32(),
        ]);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.ray_gradient(start, vector, color, color);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_gradient(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    #[inline]
    pub fn circle(&mut self, position: Vec3, normal: Vec3, radius: f32, color: Color) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        self.positions
            .extend((0..self.circle_segments).into_iter().flat_map(|i| {
                let mut angle = i as f32 * TAU / self.circle_segments as f32;
                let start = rotation * (Vec2::from(angle.sin_cos()) * radius).extend(0.) + position;

                angle += TAU / self.circle_segments as f32;
                let end = rotation * (Vec2::from(angle.sin_cos()) * radius).extend(0.) + position;

                [start.to_array(), end.to_array()]
            }));

        self.colors.extend(
            std::iter::repeat(color.as_linear_rgba_f32()).take(self.circle_segments as usize * 2),
        );
    }

    /// Draw a sphere.
    #[inline]
    pub fn sphere(&mut self, position: Vec3, radius: f32, color: Color) {
        self.circle(position, Vec3::X, radius, color);
        self.circle(position, Vec3::Y, radius, color);
        self.circle(position, Vec3::Z, radius, color);
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let half_size = size / 2.;
        let tl = (position + rotation * vec3(-half_size.x, half_size.y, 0.)).to_array();
        let tr = (position + rotation * vec3(half_size.x, half_size.y, 0.)).to_array();
        let bl = (position + rotation * vec3(-half_size.x, -half_size.y, 0.)).to_array();
        let br = (position + rotation * vec3(half_size.x, -half_size.y, 0.)).to_array();
        self.positions.extend([tl, tr, tr, br, br, bl, bl, tl]);
        self.colors
            .extend(std::iter::repeat(color.as_linear_rgba_f32()).take(8));
    }

    /// Draw a box.
    #[inline]
    pub fn cuboid(&mut self, position: Vec3, rotation: Quat, size: Vec3, color: Color) {
        let half_size = size / 2.;
        // Front
        let tlf = (position + rotation * vec3(-half_size.x, half_size.y, half_size.z)).to_array();
        let trf = (position + rotation * vec3(half_size.x, half_size.y, half_size.z)).to_array();
        let blf = (position + rotation * vec3(-half_size.x, -half_size.y, half_size.z)).to_array();
        let brf = (position + rotation * vec3(half_size.x, -half_size.y, half_size.z)).to_array();
        // Back
        let tlb = (position + rotation * vec3(-half_size.x, half_size.y, -half_size.z)).to_array();
        let trb = (position + rotation * vec3(half_size.x, half_size.y, -half_size.z)).to_array();
        let blb = (position + rotation * vec3(-half_size.x, -half_size.y, -half_size.z)).to_array();
        let brb = (position + rotation * vec3(half_size.x, -half_size.y, -half_size.z)).to_array();
        self.positions.extend([
            tlf, trf, trf, brf, brf, blf, blf, tlf, // Front
            tlb, trb, trb, brb, brb, blb, blb, tlb, // Back
            tlf, tlb, trf, trb, brf, brb, blf, blb, // Front to back
        ]);
        self.colors
            .extend(std::iter::repeat(color.as_linear_rgba_f32()).take(24));
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.line_gradient_2d(start, end, color, color);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: Color) {
        self.ray_gradient_2d(start, vector, color, color);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_gradient_2d(
        &mut self,
        start: Vec2,
        vector: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient_2d(start, start + vector, start_color, end_color);
    }

    // Draw a circle.
    #[inline]
    pub fn circle_2d(&mut self, position: Vec2, radius: f32, color: Color) {
        self.circle(position.extend(0.), Vec3::Z, radius, color);
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect_2d(&mut self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        self.rect(
            position.extend(0.),
            Quat::from_rotation_z(rotation),
            size,
            color,
        );
    }

    /// Clear everything drawn up to this point, this frame.
    #[inline]
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    /// Take the positions and colors data from `self` and overwrite the `mesh`'s vertex positions and colors.
    #[inline]
    pub fn update_mesh(&mut self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mem::take(&mut self.positions));
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, mem::take(&mut self.colors));
    }
}
