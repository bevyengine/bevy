use crate::Vec3;
use serde::{Deserialize, Serialize};

/// A ray is an infinite line starting at `origin`, going in `direction`.
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ray {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction of the ray.
    pub direction: Vec3,
}
