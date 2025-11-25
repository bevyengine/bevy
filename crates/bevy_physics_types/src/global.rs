//! Global physics tokens and constants.
//!
//! This module defines tokens that represent standard identifiers used across
//! the USD Physics schema, including degree-of-freedom names for joints and
//! stage-level metadata keys.

usd_token! {
    /// Token for the collection name used with UsdCollectionAPI to represent
    /// colliders belonging to a CollisionGroup prim.
    Colliders
}

usd_token! {
    /// Stage-level metadata key that encodes the scene's mass unit scaling.
    ///
    /// Similar to `metersPerUnit` for distance, this defines how mass values
    /// in the stage relate to kilograms. For example, `kilogramsPerUnit = 1.0`
    /// means mass values are in kilograms; `kilogramsPerUnit = 0.001` means
    /// mass values are in grams.
    KilogramsPerUnit
}

usd_token! {
    /// Token for the X translation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the transX DOF.
    TransX
}

usd_token! {
    /// Token for the Y translation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the transY DOF.
    TransY
}

usd_token! {
    /// Token for the Z translation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the transZ DOF.
    TransZ
}

usd_token! {
    /// Token for the X rotation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the rotX DOF.
    RotX
}

usd_token! {
    /// Token for the Y rotation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the rotY DOF.
    RotY
}

usd_token! {
    /// Token for the Z rotation degree of freedom.
    ///
    /// Used with [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive)
    /// to specify the rotZ DOF.
    RotZ
}

usd_token! {
    /// Token for the linear degree of freedom in prismatic joints.
    ///
    /// Used with prismatic joint drives to specify the single translation DOF.
    Linear
}

usd_token! {
    /// Token for the angular degree of freedom in revolute joints.
    ///
    /// Used with revolute joint drives to specify the single rotation DOF.
    Angular
}

usd_token! {
    /// Token for the distance limit in generic D6 joints.
    ///
    /// Used to specify a distance constraint (min/max distance between
    /// joint anchor points).
    Distance
}
