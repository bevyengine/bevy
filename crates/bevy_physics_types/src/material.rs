//! Physics material properties for collision response.
//!
//! The [`DynamicFriction`], [`StaticFriction`], [`Restitution`], and [`Density`]
//! properties define physical material characteristics that affect how objects
//! interact during collisions.
//!
//! ## Material Binding
//!
//! Physics materials are bound the same way as graphics materials, either with
//! no material purpose or with a specific "physics" purpose. This allows adding
//! physics properties to an established material library.
//!
//! Materials can be bound to geometry subsets, though not all physics
//! implementations support different materials per subset. Unsupported
//! implementations may use only one material per collider.
//!
//! ## Friction Model
//!
//! The friction coefficients use the **Coulomb friction model**:
//!
//! - [`StaticFriction`]: Coefficient of friction when surfaces are not sliding
//!   relative to each other. Objects won't start sliding until the applied
//!   tangential force exceeds `staticFriction × normalForce`.
//!
//! - [`DynamicFriction`]: Coefficient of friction when surfaces are sliding.
//!   The friction force equals `dynamicFriction × normalForce`.
//!
//! ## Coefficient of Restitution
//!
//! [`Restitution`] is the ratio of final to initial relative velocity between
//! two objects after collision:
//! - `0.0` = perfectly inelastic (objects stick together, no bounce)
//! - `1.0` = perfectly elastic (full energy preserved, maximum bounce)
//!
//! ## Combine Modes
//!
//! Friction and restitution are ideally defined for pairs of materials, but
//! this is impractical. Real-time physics typically defines them per-material
//! and combines them using a formula. The default behavior is to **average**
//! the values from both materials (consistent with popular game engines).
//!
//! Future versions may expose other combine modes (product, minimum, maximum).
//!
//! ## Density
//!
//! Material [`Density`] provides a way to set density for mass computation,
//! but it has the **lowest precedence** in density resolution:
//! 1. Explicit density via [`MassAPI`](crate::mass::Density) (highest)
//! 2. Inherited density from parent
//! 3. Material density (lowest)

use crate::types::float;

make_asset! {
    /// Physics material asset defining collision response properties.
    ///
    /// All colliders with a relationship to this material will have their
    /// collision response defined by these properties.
    Material
}

make_attribute! {
    /// Dynamic (kinetic) friction coefficient.
    ///
    /// Applied when surfaces are sliding relative to each other. The friction
    /// force opposes motion and equals `dynamicFriction × normalForce`.
    ///
    /// Typical values:
    /// - Ice on ice: ~0.03
    /// - Rubber on concrete: ~0.6-0.8
    /// - Steel on steel: ~0.4-0.6
    ///
    /// Unitless. Range: [0, ∞) though typically [0, 1].
    DynamicFriction(float) = 0.0;
    apiName = "dynamicFriction"
    displayName = "Dynamic Friction"
}

make_attribute! {
    /// Static friction coefficient.
    ///
    /// Applied when surfaces are not sliding. An object won't start sliding
    /// until the tangential force exceeds `staticFriction × normalForce`.
    /// Static friction is typically higher than dynamic friction.
    ///
    /// Unitless. Range: [0, ∞) though typically [0, 1].
    StaticFriction(float) = 0.0;
    apiName = "staticFriction"
    displayName = "Static Friction"
}

make_attribute! {
    /// Coefficient of restitution (bounciness).
    ///
    /// Ratio of separation velocity to approach velocity after collision:
    /// - `0.0`: Perfectly inelastic, no bounce
    /// - `1.0`: Perfectly elastic, full bounce
    ///
    /// Values above 1.0 add energy (not physically realistic but sometimes
    /// useful for gameplay).
    ///
    /// Unitless. Range: [0, 1] for realistic behavior.
    Restitution(float) = 0.0;
    apiName = "restitution"
    displayName = "Restitution"
}

make_attribute! {
    /// Material density for implicit mass computation.
    ///
    /// If non-zero, this density can be used for body mass computation when
    /// no explicit mass or density is specified via [`MassAPI`](crate::mass).
    ///
    /// **Note**: Material density has the weakest precedence in density
    /// definition—it is overridden by any density specified via MassAPI.
    ///
    /// A value of 0.0 means "not specified" and is ignored.
    ///
    /// Units: mass/distance³.
    Density(float) = 0.0;
    apiName = "density"
    displayName = "Density"
}
