mod face_toward;
mod perspective;

pub use face_toward::*;
pub use glam::*;
pub use perspective::*;

pub mod prelude {
    pub use crate::{FaceToward, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
}
