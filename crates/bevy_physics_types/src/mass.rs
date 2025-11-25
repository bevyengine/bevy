//! Mass properties for physics objects.
//!
//! The PhysicsMassAPI defines explicit mass properties including mass, density,
//! center of mass, and inertia tensor. This API can be applied to any object
//! that has a PhysicsCollisionAPI or PhysicsRigidBodyAPI. Mass has precedence
//! over density when both are specified. Child prims' density overrides parent
//! density as it is accumulative, while parent mass overrides child mass. The
//! inertia tensor is specified as diagonal components along principal axes with
//! an optional orientation quaternion.
use bevy_math::prelude::*;

usd_attribute! {
    /// If non-zero, directly specifies the mass of the object.
    /// Note that any child prim can also have a mass when they apply massAPI.
    /// In this case, the precedence rule is 'parent mass overrides the
    /// child's'. This may come as counter-intuitive, but mass is a computed
    /// quantity and in general not accumulative. For example, if a parent
    /// has mass of 10, and one of two children has mass of 20, allowing
    /// child's mass to override its parent results in a mass of -10 for the
    /// other child. Note if mass is 0.0 it is ignored. Units: mass.
    Mass(f32) = 0.0;
    apiName = "mass"
    displayName = "Mass"
}

usd_attribute! {
    /// If non-zero, specifies the density of the object.
    /// In the context of rigid body physics, density indirectly results in
    /// setting mass via (mass = density x volume of the object). How the
    /// volume is computed is up to implementation of the physics system.
    /// It is generally computed from the collision approximation rather than
    /// the graphical mesh. In the case where both density and mass are
    /// specified for the same object, mass has precedence over density.
    /// Unlike mass, child's prim's density overrides parent prim's density
    /// as it is accumulative. Note that density of a collisionAPI can be also
    /// alternatively set through a PhysicsMaterialAPI. The material density
    /// has the weakest precedence in density definition. Note if density is
    /// 0.0 it is ignored. Units: mass/distance/distance/distance.
    Density(f32) = 0.0;
    apiName = "density"
    displayName = "Density"
}

usd_attribute! {
    /// Center of mass in the prim's local space. Units: distance.
    CenterOfMass(Vec3) = vec3(-9999999.0, -9999999.0, -9999999.0);
    apiName = "centerOfMass"
    displayName = "Center of Mass"
}

usd_attribute! {
    /// If non-zero, specifies diagonalized inertia tensor along the
    /// principal axes. Note if diagonalInertial is (0.0, 0.0, 0.0) it is
    /// ignored. Units: mass*distance*distance.
    DiagonalInertia(Vec3) = vec3(0.0, 0.0, 0.0);
    apiName = "diagonalInertia"
    displayName = "Diagonal Inertia"
}

usd_attribute! {
    /// Orientation of the inertia tensor's principal axes in the
    /// prim's local space.
    PrincipalAxes(Quat) = quat(0.0, 0.0, 0.0, 0.0);
    apiName = "principalAxes"
    displayName = "Principal Axes"
}
