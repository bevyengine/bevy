//! Global physics tokens and constants.
//!
//! This module defines global identifiers used across the physics system,
//! including degree-of-freedom names for joints and scene-level metadata keys.

use crate::axis::Axis;
use crate::types::float;

make_global! {
    /// Scene-level metadata that encodes the distance unit scaling.
    ///
    /// Defines how distance values in the scene relate to meters.
    /// For example, `metersPerUnit = 1.0` means distance values are in meters;
    /// `metersPerUnit = 0.01` means distance values are in centimeters.
    ///
    /// This affects gravity magnitude and other physics calculations.
    MetersPerUnit(float) = 1.0;
    apiName = "metersPerUnit"
    displayName = "Meters Per Unit"
}

make_global! {
    /// Scene-level metadata that specifies the up axis.
    ///
    /// Defines which axis represents "up" in the coordinate system.
    /// This affects default gravity direction and other orientation-dependent
    /// physics behaviors.
    UpAxis(Axis) = Axis::Y;
    apiName = "upAxis"
    displayName = "Up Axis"
}

make_global! {
    /// Scene-level metadata that encodes the mass unit scaling.
    ///
    /// Similar to `metersPerUnit` for distance, this defines how mass values
    /// in the scene relate to kilograms. For example, `kilogramsPerUnit = 1.0`
    /// means mass values are in kilograms; `kilogramsPerUnit = 0.001` means
    /// mass values are in grams.
    KilogramsPerUnit(float) = 1.0;
    apiName = "kilogramsPerUnit"
    displayName = "Kilograms Per Unit"
}
