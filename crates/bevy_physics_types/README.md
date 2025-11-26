# USD Physics Schema for Bevy

This crate implements the [USD Physics Schema](https://openusd.org/release/api/usd_physics_page_front.html)
as Bevy ECS components, enabling rigid body physics simulation representation compatible
with the Universal Scene Description (USD) standard.

## Purpose and Scope

While USD was primarily targeted at film and VFX pipelines, it has been adopted into
many other domains including real-time physics for games, robotics, mechanical engineering,
and AI/robotics training simulations. This crate provides the baseline physics representation
for rigid body physics as defined by the USD Physics specification.

## Units

Physics uses USD's established concepts of distance and time, plus mass:
- **Distance**: Arbitrary units with `metersPerUnit` metadata for scaling
- **Time**: Seconds
- **Mass**: Arbitrary units with `kilogramsPerUnit` metadata for scaling
- **Angles**: Degrees (consistent with USD conventions like `UsdGeomPointInstancer`)

All physical quantities can be decomposed into products of these three basic types.

## Core Concepts

- **scene**: Physics simulation scenes with gravity configuration
- **rigid_body**: Dynamic and kinematic rigid body properties
- **mass**: Mass, density, center of mass, and inertia configuration
- **collision**: Collision shape markers for physics interaction
- **mesh_collision**: Mesh-to-collider approximation methods
- **material**: Friction, restitution, and density material properties
- **collision_group**: Coarse-grained collision filtering groups
- **filtered_pairs**: Fine-grained pairwise collision filtering
- **joint**: Joint constraints between rigid bodies
- **articulation**: Reduced coordinate articulated body hierarchies

## Hierarchy Behavior

When a prim has `RigidBody` applied, all prims in its subtree move rigidly with it,
except descendants that have their own `RigidBody` which form independent bodies.
Collision shapes (`CollisionEnabled`) under a rigid body become part of that body's
collision representation.

## References

- [USD Physics Schema Documentation](https://openusd.org/release/api/usd_physics_page_front.html)
- [USD Physics Schema Source](https://github.com/PixarAnimationStudios/OpenUSD/blob/dev/pxr/usd/usdPhysics/schema.usda)