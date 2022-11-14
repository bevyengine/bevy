use std::{f32::consts::TAU, iter};

use bevy_asset::Handle;
use bevy_ecs::system::Resource;
use bevy_math::{vec3, Quat, Vec2, Vec3};
use bevy_render::prelude::{Color, Mesh};

/// A resource with an immediate mode API for drawing lines and wireshapes.
/// Useful for visual debugging.
#[derive(Resource)]
pub struct DebugDraw {
    pub(crate) list_mesh_handle: Option<Handle<Mesh>>,
    pub(crate) list_positions: Vec<[f32; 3]>,
    pub(crate) list_colors: Vec<[f32; 4]>,
    pub(crate) strip_mesh_handle: Option<Handle<Mesh>>,
    pub(crate) strip_positions: Vec<[f32; 3]>,
    pub(crate) strip_colors: Vec<[f32; 4]>,
    /// The amount of line segments to use when drawing a circle.
    ///
    /// Defaults to `32`.
    pub circle_segments: u32,
}

impl Default for DebugDraw {
    fn default() -> Self {
        DebugDraw {
            list_mesh_handle: None,
            list_positions: Vec::new(),
            list_colors: Vec::new(),
            strip_mesh_handle: None,
            strip_positions: Vec::new(),
            strip_colors: Vec::new(),
            circle_segments: 32,
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
        self.list_positions
            .extend([start.to_array(), end.to_array()]);
        self.list_colors.extend([
            start_color.as_linear_rgba_f32(),
            end_color.as_linear_rgba_f32(),
        ]);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.line(start, start + vector, color);
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

        let positions = self
            .circle_inner(radius)
            .map(|vec2| (rotation * vec2.extend(0.) + position).to_array())
            .chain(iter::once(Vec3::NAN.to_array()));

        self.strip_positions.extend(positions);
        self.add_strip_color(color, (self.circle_segments + 1) as usize);
    }

    /// Draw a sphere.
    #[inline]
    pub fn sphere(&mut self, position: Vec3, radius: f32, color: Color) {
        self.strip_colors
            .reserve((self.circle_segments + 1) as usize * 3);
        self.strip_positions
            .reserve((self.circle_segments + 1) as usize * 3);
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

        self.strip_positions
            .extend([tl, tr, br, bl, tl, [f32::NAN; 3]]);
        self.add_strip_color(color, 5);
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

        self.list_positions.extend([
            tlf, trf, trf, brf, brf, blf, blf, tlf, // Front
            tlb, trb, trb, brb, brb, blb, blb, tlb, // Back
            tlf, tlb, trf, trb, brf, brb, blf, blb, // Front to back
        ]);
        self.add_list_color(color, 24);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.line(start.extend(0.), end.extend(0.), color);
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
        self.line_2d(start, start + vector, color);
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
        let positions = self
            .circle_inner(radius)
            .map(|vec2| (vec2 + position).extend(0.).to_array())
            .chain(iter::once([f32::NAN; 3]));

        self.strip_positions.extend(positions);
        self.add_strip_color(color, (self.circle_segments + 1) as usize);
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

    #[inline]
    fn add_strip_color(&mut self, color: Color, amount: usize) {
        self.strip_colors.extend(
            iter::repeat(color.as_linear_rgba_f32())
                .take(amount)
                .chain(iter::once([f32::NAN; 4])),
        );
    }

    #[inline]
    fn add_list_color(&mut self, color: Color, amount: usize) {
        self.list_colors
            .extend(iter::repeat(color.as_linear_rgba_f32()).take(amount));
    }

    fn circle_inner(&self, radius: f32) -> impl Iterator<Item = Vec2> {
        let circle_segments = self.circle_segments;
        (0..(circle_segments + 1)).into_iter().map(move |i| {
            let angle = i as f32 * TAU / circle_segments as f32;
            Vec2::from(angle.sin_cos()) * radius
        })
    }

    /// Clear everything drawn up to this point, this frame.
    #[inline]
    pub fn clear(&mut self) {
        self.list_positions.clear();
        self.list_colors.clear();
        self.strip_positions.clear();
        self.strip_colors.clear();
    }
}
