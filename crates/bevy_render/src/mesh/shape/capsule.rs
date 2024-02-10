use crate::mesh::{Capsule3dMeshBuilder, CapsuleUvProfile, Mesh};

/// A cylinder with hemispheres at the top and bottom
#[deprecated(
    since = "0.13.0",
    note = "please use the `Capsule3d` primitive in `bevy_math` instead"
)]
#[derive(Debug, Copy, Clone)]
pub struct Capsule {
    /// Radius on the `XZ` plane.
    pub radius: f32,
    /// Number of sections in cylinder between hemispheres.
    pub rings: usize,
    /// Height of the middle cylinder on the `Y` axis, excluding the hemispheres.
    pub depth: f32,
    /// Number of latitudes, distributed by inclination. Must be even.
    pub latitudes: usize,
    /// Number of longitudes, or meridians, distributed by azimuth.
    pub longitudes: usize,
    /// Manner in which UV coordinates are distributed vertically.
    pub uv_profile: CapsuleUvProfile,
}
impl Default for Capsule {
    fn default() -> Self {
        Capsule {
            radius: 0.5,
            rings: 0,
            depth: 1.0,
            latitudes: 16,
            longitudes: 32,
            uv_profile: CapsuleUvProfile::Aspect,
        }
    }
}

impl From<Capsule> for Mesh {
    #[allow(clippy::needless_range_loop)]
    fn from(capsule: Capsule) -> Self {
        Capsule3dMeshBuilder::new(
            capsule.radius,
            capsule.depth,
            capsule.longitudes,
            capsule.latitudes,
        )
        .rings(capsule.rings)
        .uv_profile(capsule.uv_profile)
        .build()
    }
}
