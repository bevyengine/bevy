//! Articulated rigid body hierarchies (reduced coordinate systems).
//!
//! The [`ArticulationRoot`] marker identifies a subtree for reduced coordinate
//! articulation solving. This is an optimization for tree-structured joint
//! hierarchies (like robot arms or ragdolls) that can be solved more efficiently
//! and stably than general maximal coordinate constraints.
//!
//! ## Maximal vs Reduced Coordinates
//!
//! - **Maximal coordinates**: Each body's pose is independent; joints apply
//!   constraint forces to maintain relationships. General but can drift.
//! - **Reduced coordinates**: Bodies are parameterized by joint angles/positions
//!   relative to parents. More stable for tree-structured hierarchies.
//!
//! This API instructs the simulation to prefer reduced coordinate solving for
//! the marked subtree.
//!
//! ## Floating vs Fixed Articulations
//!
//! In robotics terminology:
//!
//! - **Floating articulation**: The root body is free to move in the world
//!   (e.g., a wheeled robot, quadcopter, humanoid). Place [`ArticulationRoot`]
//!   on the root body (typically the central mass) or a parent entity.
//!
//! - **Fixed articulation**: The root is bolted to the world (e.g., an industrial
//!   robot arm, conveyor mechanism). Place [`ArticulationRoot`] on a parent of
//!   the root joint that connects to world, or on that joint itself.
//!
//! ## Loop Handling
//!
//! Reduced coordinate systems require tree topology—no loops. If loops exist:
//!
//! 1. The implementation may break loops at an arbitrary location
//! 2. A joint in the loop can use [`ExcludeFromArticulation`](crate::joint::ExcludeFromArticulation)
//!    to explicitly remain a maximal joint, breaking the loop there
//!
//! Joints excluded from articulations are still simulated but use maximal
//! coordinate constraints.
//!
//! ## Kinematic Articulations
//!
//! An articulation can be made kinematic (driven by animation/motion capture)
//! by setting [`Kinematic`](crate::rigid_body::Kinematic) on all bodies within
//! the articulation.
//!
//! ## Multiple Roots
//!
//! If multiple qualifying bodies or joints are found while parsing the subtree,
//! each becomes a separate articulation root. Nested articulation root markers
//! are not allowed and will produce errors.
//!
//! ## Example Structures
//!
//! - **Robot arm**: Fixed joint to world → ArticulationRoot → chain of revolute joints
//! - **Ragdoll**: ArticulationRoot on pelvis → joints to spine, legs, arms
//! - **Vehicle**: ArticulationRoot on chassis → wheel joints

make_marker! {
    /// Marks this subtree as an articulated rigid body hierarchy.
    ///
    /// Any joints found in this subtree will preferentially be simulated
    /// using reduced coordinate algorithms for improved stability and
    /// performance on tree-structured hierarchies.
    ///
    /// For floating articulations (not attached to world), place this on
    /// the root body. For fixed articulations, place on a parent of the
    /// root joint or on the joint itself.
    ArticulationRoot;
    apiName = "articulationRootApi"
    displayName = "Articulation Root"
}
