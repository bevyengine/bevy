mod clamp;
mod face_toward;
mod geometry;
mod map_range;

pub use clamp::*;
pub use face_toward::*;
pub use geometry::*;
pub use map_range::*;
pub use glam::*;

pub mod prelude {
    pub use crate::{FaceToward, Mat3, Mat4, Quat, Rect, Size, Vec2, Vec3, Vec4};
}
