mod face_toward;
mod geometry;
mod smooth_damp;

pub use face_toward::*;
pub use geometry::*;
pub use glam::*;
pub use smooth_damp::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        BVec2, BVec3, BVec4, FaceToward, IVec2, IVec3, IVec4, Mat3, Mat4, Quat, Rect, Size,
        SmoothDamp, SmoothDampFunctions, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
    };
}
