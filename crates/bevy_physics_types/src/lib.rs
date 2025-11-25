//! # USD Physics Schema for Bevy
//!
//! This crate implements the [USD Physics Schema](https://openusd.org/release/api/usd_physics_page_front.html)
//! as Bevy ECS components, enabling rigid body physics simulation representation compatible
//! with the Universal Scene Description (USD) standard.
//!
//! ## Purpose and Scope
//!
//! While USD was primarily targeted at film and VFX pipelines, it has been adopted into
//! many other domains including real-time physics for games, robotics, mechanical engineering,
//! and AI/robotics training simulations. This crate provides the baseline physics representation
//! for rigid body physics as defined by the USD Physics specification.
//!
//! ## Overall Design
//!
//! The schema follows USD's design philosophy of attaching API schemas to existing objects
//! rather than creating new prim types whenever possible. This minimizes scene graph inflation
//! while enabling physics behavior to be added to or removed from existing content.
//!
//! ## Units
//!
//! Physics uses USD's established concepts of distance and time, plus mass:
//! - **Distance**: Arbitrary units with `metersPerUnit` metadata for scaling
//! - **Time**: Seconds (consistent with `timeCodesPerSecond`)
//! - **Mass**: Arbitrary units with `kilogramsPerUnit` metadata for scaling
//! - **Angles**: Degrees (consistent with USD conventions like `UsdGeomPointInstancer`)
//!
//! All physical quantities can be decomposed into products of these three basic types.
//!
//! ## Core Concepts
//!
//! - **[`scene`]**: Physics simulation scenes with gravity configuration
//! - **[`rigid_body`]**: Dynamic and kinematic rigid body properties
//! - **[`mass`]**: Mass, density, center of mass, and inertia configuration
//! - **[`collision`]**: Collision shape markers for physics interaction
//! - **[`mesh_collision`]**: Mesh-to-collider approximation methods
//! - **[`material`]**: Friction, restitution, and density material properties
//! - **[`collision_group`]**: Coarse-grained collision filtering groups
//! - **[`filtered_pairs`]**: Fine-grained pairwise collision filtering
//! - **[`joint`]**: Joint constraints between rigid bodies
//! - **[`articulation`]**: Reduced coordinate articulated body hierarchies
//!
//! ## Hierarchy Behavior
//!
//! When a prim has `RigidBody` applied, all prims in its subtree move rigidly with it,
//! except descendants that have their own `RigidBody` which form independent bodies.
//! Collision shapes (`CollisionEnabled`) under a rigid body become part of that body's
//! collision representation.

#[macro_use]
mod macros;

pub mod axis;

pub mod global;

pub mod scene;

pub mod types;

pub mod rigid_body;

pub mod mass;

pub mod collision;

pub mod mesh_collision;

pub mod material;

pub mod collision_group;

pub mod filtered_pairs;

pub mod joint;

pub mod joint_revolute;

pub mod joint_prismatic;

pub mod joint_spherical;

pub mod joint_distance;

pub mod joint_fixed;

pub mod limit;

pub mod drive;

pub mod articulation;
