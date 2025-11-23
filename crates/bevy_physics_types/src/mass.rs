use bevy_ecs::prelude::Component;
use bevy_math::{primitives::{Capsule3d, Cuboid, Cylinder, Plane3d, Sphere}, Quat, Vec3};



/// Directly specifies the mass of the object.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Mass(pub f32);

/// Specifies the density of the object.
/// unlike mass, child prim density overrides parent prim density.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Density(pub f32);

impl Default for Density {
    fn default() -> Self {
        Self(1000.0)
    }
}

/// Center of mass in the prim's local space.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CenterOfMass(pub Vec3);

impl Default for CenterOfMass {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Orientation of the inertia tensor's principal axes in the prim's local space.
/// USD `physics:principalAxes`
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct InertiaOrientation(pub Quat);

impl Default for InertiaOrientation {
    fn default() -> Self {
        Self(Quat::IDENTITY)
    }
}
/// specifies diagonalized inertia tensor along the principal axes
/// Units: `mass*distance*distance`
/// USD `physics:diagonalInertia`
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PrincipalInertia(pub Vec3);

impl Default for PrincipalInertia {
    fn default() -> Self {
        Self(Vec3::splat(1.0))
    }
}
