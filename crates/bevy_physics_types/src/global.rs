//! Global physics tokens and constants.
//!
//! This module defines tokens that represent standard identifiers used across
//! the USD Physics schema, including degree-of-freedom names for joints and
//! stage-level metadata keys.

use crate::axis::Axis;

usd_global! {
    /// Stage-level metadata that encodes the scene's distance unit scaling.
    ///
    /// Defines how distance values in the stage relate to meters.
    /// For example, `metersPerUnit = 1.0` means distance values are in meters;
    /// `metersPerUnit = 0.01` means distance values are in centimeters.
    ///
    /// This affects gravity magnitude and other physics calculations.
    MetersPerUnit(f32) = 1.0;
    apiName = "metersPerUnit"
    displayName = "Meters Per Unit"
}

usd_global! {
    /// Stage-level metadata that specifies the up axis for the scene.
    ///
    /// Defines which axis represents "up" in the coordinate system.
    /// This affects default gravity direction and other orientation-dependent
    /// physics behaviors.
    UpAxis(Axis) = Axis::Y;
    apiName = "upAxis"
    displayName = "Up Axis"
}

usd_global! {
    /// Stage-level metadata that encodes the scene's mass unit scaling.
    ///
    /// Similar to `metersPerUnit` for distance, this defines how mass values
    /// in the stage relate to kilograms. For example, `kilogramsPerUnit = 1.0`
    /// means mass values are in kilograms; `kilogramsPerUnit = 0.001` means
    /// mass values are in grams.
    KilogramsPerUnit(f32) = 1.0;
    apiName = "kilogramsPerUnit"
    displayName = "Kilograms Per Unit"
}
