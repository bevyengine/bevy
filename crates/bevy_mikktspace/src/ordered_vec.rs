use glam::Vec3;
use ordered_float::NotNan;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct FiniteVec3 {
    x: NotNan<f32>,
    y: NotNan<f32>,
    z: NotNan<f32>,
}

impl FiniteVec3 {
    pub fn new(val: Vec3) -> Result<FiniteVec3, NotFinite> {
        if val.is_finite() {
            Ok(Self {
                // Unwrapping here is fine, because is_finite guarantees that
                // the values are not `Nan`
                x: NotNan::new(val.x).unwrap(),
                y: NotNan::new(val.y).unwrap(),
                z: NotNan::new(val.z).unwrap(),
            })
        } else {
            Err(NotFinite)
        }
    }
}

#[derive(Debug)]
pub struct NotFinite;
