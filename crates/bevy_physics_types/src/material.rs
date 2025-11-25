//! Material properties for physics simulations.

usd_asset! {
    /// All collisions that have a relationship to this material will have their
    /// collision response defined through this material. Material properties include
    /// dynamic and static friction coefficients, restitution (bounciness), and
    /// density which can be used for body mass computation.
    Material
}

usd_attribute! {
    /// Dynamic friction coefficient. Unitless.
    DynamicFriction(f32) = 0.0;
    apiName = "dynamicFriction"
    displayName = "Dynamic Friction"
}

usd_attribute! {
    /// Static friction coefficient. Unitless.
    StaticFriction(f32) = 0.0;
    apiName = "staticFriction"
    displayName = "Static Friction"
}

usd_attribute! {
    /// Restitution coefficient. Unitless.
    Restitution(f32) = 0.0;
    apiName = "restitution"
    displayName = "Restitution"
}

usd_attribute! {
    /// If non-zero, defines the density of the material. This can be
    /// used for body mass computation, see PhysicsMassAPI.
    /// Note that if the density is 0.0 it is ignored.
    /// Units: mass/distance/distance/distance.
    Density(f32) = 0.0;
    apiName = "density"
    displayName = "Density"
}
