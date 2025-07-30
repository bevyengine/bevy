//! Experimental Gizmo API for Bevy
//!
//! This crate provides a flexible, composable, and ECS-friendly gizmo system inspired by the issue proposal.
//!
//! ## Usage Example (in a Bevy app)
//! ```
//! use bevy::prelude::*;
//! use bevy_gizmos_experimental::*;
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(DrawGizmo(Circle { center: Vec3::ZERO, radius: 1.0 }.gizmo(Color::WHITE)));
//!     commands.spawn(DrawGizmo(Line3d { start: Vec3::ZERO, end: Vec3::X }.gizmo(Color::RED)));
//! }
//! ```

use std::ops::Deref;

// Re-export for convenience

pub use bevy_color::Color;
pub use bevy_ecs::{bundle::Bundle, component::Component, system::Query};
pub use bevy_math::{Vec2, Vec3};

/// A simple, composable gizmo representation.
///
/// This struct can represent any gizmo shape as a set of points and colors.
/// You may extend this with topology, thickness, etc. as needed.
#[derive(Clone, Debug, PartialEq)]
pub struct Gizmo {
    pub points: Vec<Vec3>,
    pub colors: Vec<Color>,
    // Optionally: pub topology: GizmoTopology,
    // Optionally: pub thickness: f32,
}

impl Gizmo {
    /// Merge another gizmo into this one, combining their points and colors.
    pub fn merge(mut self, other: Gizmo) -> Self {
        self.points.extend(other.points);
        self.colors.extend(other.colors);
        self
    }
}

/// Trait for types that can be converted into a Gizmo.
///
/// This is analogous to how meshes are handled in Bevy.
pub trait Gizmoable {
    fn gizmo(&self, color: impl Into<Color>) -> Gizmo;
}

/// 2D Circle (in the XY plane)
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec3, // Z is ignored for 2D, but allows 3D placement
    pub radius: f32,
    pub resolution: u32, // Number of segments
}

impl Default for Circle {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 1.0,
            resolution: 32,
        }
    }
}

impl Gizmoable for Circle {
    fn gizmo(&self, color: impl Into<Color>) -> Gizmo {
        let color = color.into();
        let points: Vec<Vec3> = (0..=self.resolution)
            .map(|i| {
                let theta = (i as f32) * std::f32::consts::TAU / (self.resolution as f32);
                self.center + Vec3::new(theta.cos() * self.radius, theta.sin() * self.radius, 0.0)
            })
            .collect();
        Gizmo {
            points,
            colors: vec![color; (self.resolution + 1) as usize],
        }
    }
}

/// 3D Line
#[derive(Clone, Debug, PartialEq)]
pub struct Line3d {
    pub start: Vec3,
    pub end: Vec3,
}

impl Gizmoable for Line3d {
    fn gizmo(&self, color: impl Into<Color>) -> Gizmo {
        let color = color.into();
        Gizmo {
            points: vec![self.start, self.end],
            colors: vec![color, color],
        }
    }
}

/// 2D Line (in the XY plane)
#[derive(Clone, Debug, PartialEq)]
pub struct Line2d {
    pub start: Vec2,
    pub end: Vec2,
    pub z: f32, // Z-plane for placement
}

impl Gizmoable for Line2d {
    fn gizmo(&self, color: impl Into<Color>) -> Gizmo {
        let color = color.into();
        Gizmo {
            points: vec![
                Vec3::new(self.start.x, self.start.y, self.z),
                Vec3::new(self.end.x, self.end.y, self.z),
            ],
            colors: vec![color, color],
        }
    }
}

// This should be treated as bundle instead.
/// ECS Component for drawing a gizmo
#[derive(Clone, Debug, PartialEq, bevy_ecs::component::Component)]
pub struct DrawGizmos(pub Gizmo);

/// System to collect and draw all DrawGizmo components (stub for integration)
pub fn draw_gizmo_components(query: bevy_ecs::system::Query<&DrawGizmos>) {
    for draw_gizmo in &query {
        // Here you would send the points/colors to your renderer or Bevy's gizmo system
        // For now, we just print them for demonstration
        println!("Drawing gizmo: {:?}", draw_gizmo.0);
    }
}
