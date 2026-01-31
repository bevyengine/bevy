//! Mass properties for physics objects.
//!
//! The [`Mass`], [`Density`], [`CenterOfMass`], and inertia properties define how
//! mass is distributed within a rigid body. This API can be applied to any entity
//! that has a [`CollisionEnabled`](crate::collision::CollisionEnabled) or
//! [`RigidBody`](crate::rigid_body::RigidBody).
//!
//! ## Mass Precedence Rules
//!
//! Mass can be specified in multiple ways, resolved using this precedence system:
//!
//! 1. **Explicit mass on rigid body** overrides any mass properties in its subtree.
//!
//! 2. **Mass overrides density**: Explicit mass always takes precedence over
//!    implicit mass computed from volume × density.
//!
//! 3. **Child density overrides parent**: A density on a child collider overrides
//!    a density specified on a parent rigid body for that child.
//!
//! 4. **MassAPI density overrides material density**: Density specified via this
//!    API (even if inherited from a parent) overrides any density specified via
//!    [`PhysicsMaterialAPI`](crate::material).
//!
//! 5. **Implicit mass computation**: A collider's implicit mass equals its
//!    computed volume times the locally effective density.
//!
//! 6. **Rigid body implicit mass**: Total implicit mass of all collision shapes
//!    in the subtree belonging to that body.
//!
//! ## Default Values
//!
//! - **Default density**: 1000.0 kg/m³ (approximately water density) when no
//!   density is specified locally or via bound materials. This value is converted
//!   to the collider's native units before mass computation.
//!
//! - **Default mass**: 1.0 in scene mass units when none is provided explicitly
//!   and there are no collision volumes to derive from.
//!
//! ## Sentinel Values
//!
//! A value of 0.0 for [`Mass`] or [`Density`] means "not specified" and the
//! attribute is ignored. For [`CenterOfMass`], a sentinel value outside the
//! normal range indicates automatic computation from collision geometry.

use crate::types::{float, point3f, quatf, vector3f};

make_attribute! {
    /// Explicit mass of the object.
    ///
    /// If non-zero, directly specifies the mass. Note that child entities can
    /// also have mass when they apply MassAPI. The precedence rule is:
    /// **parent mass overrides child mass**.
    ///
    /// This may seem counter-intuitive, but mass is a computed quantity and
    /// generally not accumulative. For example, if a parent has mass 10 and
    /// one of two children has mass 20, allowing child mass to override would
    /// result in -10 mass for the other child.
    ///
    /// A value of 0.0 means "not specified" and is ignored.
    ///
    /// Units: mass (scaled by scene `kilogramsPerUnit`).
    Mass(float) = 0.0;
    apiName = "mass"
    displayName = "Mass"
}

make_attribute! {
    /// Density of the object for implicit mass computation.
    ///
    /// If non-zero, specifies the density. In rigid body physics, density
    /// indirectly sets mass via: `mass = density × volume`. The volume is
    /// typically computed from collision geometry rather than render geometry.
    ///
    /// When both density and mass are specified, **mass takes precedence**.
    ///
    /// Unlike mass, **child density overrides parent density** as density is
    /// accumulative through the hierarchy.
    ///
    /// Density can also be set via [`PhysicsMaterialAPI`](crate::material),
    /// but MassAPI density has higher precedence than material density.
    ///
    /// A value of 0.0 means "not specified" and is ignored.
    /// Default when unspecified: 1000.0 kg/m³ (water density).
    ///
    /// Units: mass/distance³.
    Density(float) = 0.0;
    apiName = "density"
    displayName = "Density"
}

make_attribute! {
    /// Center of mass in the entity's local space.
    ///
    /// When specified, overrides the automatically computed center of mass.
    /// The sentinel value (very large negative numbers) indicates that the
    /// center of mass should be computed from the collision geometry.
    ///
    /// Units: distance.
    CenterOfMass(point3f) = point3f::splat(-9999999.0);
    apiName = "centerOfMass"
    displayName = "Center of Mass"
}

make_attribute! {
    /// Diagonalized inertia tensor along principal axes.
    ///
    /// If non-zero, specifies the diagonal components of the inertia tensor
    /// when expressed in the principal axes frame (see [`PrincipalAxes`]).
    /// The inertia tensor describes how mass is distributed and affects
    /// rotational dynamics.
    ///
    /// A value of (0, 0, 0) means "not specified" and inertia should be
    /// computed from collision geometry and mass distribution.
    ///
    /// Units: mass × distance².
    DiagonalInertia(vector3f) = vector3f::ZERO;
    apiName = "diagonalInertia"
    displayName = "Diagonal Inertia"
}

make_attribute! {
    /// Orientation of the inertia tensor's principal axes.
    ///
    /// Specifies the rotation from the entity's local space to the principal
    /// axes frame in which [`DiagonalInertia`] is expressed.
    ///
    /// Unitless (quaternion).
    PrincipalAxes(quatf) = quatf::IDENTITY;
    apiName = "principalAxes"
    displayName = "Principal Axes"
}
