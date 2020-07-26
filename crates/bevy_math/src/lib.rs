mod face_toward;
mod geometry;
mod perspective;

pub use face_toward::*;
pub use geometry::*;
pub use glam::*;
pub use perspective::*;

pub mod prelude {
    pub use crate::{FaceToward, Mat3, Mat4, Quat, Rect, Size, Vec2, Vec3, Vec4};
}
