use crate::mesh::{CylinderMeshBuilder, Mesh};

/// A cylinder which stands on the XZ plane
#[deprecated(
    since = "0.13.0",
    note = "please use the `Cylinder` primitive in `bevy_math` instead"
)]
#[derive(Clone, Copy, Debug)]
pub struct Cylinder {
    /// Radius in the XZ plane.
    pub radius: f32,
    /// Height of the cylinder in the Y axis.
    pub height: f32,
    /// The number of vertices around each horizontal slice of the cylinder. If you are looking at the cylinder from
    /// above, this is the number of points you will see on the circle.
    /// A higher number will make it appear more circular.
    pub resolution: u32,
    /// The number of segments between the two ends. Setting this to 1 will have triangles spanning the full
    /// height of the cylinder. Setting it to 2 will have two sets of triangles with a horizontal slice in the middle of
    /// cylinder. Greater numbers increase triangles/slices in the same way.
    pub segments: u32,
}

impl Default for Cylinder {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 1.0,
            resolution: 16,
            segments: 1,
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(c: Cylinder) -> Self {
        CylinderMeshBuilder::new(c.radius, c.height, c.resolution)
            .segments(c.segments)
            .build()
    }
}
