# Physics Types for Bevy

This crate provides physics component types for Bevy ECS, enabling rigid body
physics simulation representation.

## Purpose and Scope

This crate provides the baseline physics representation for rigid body physics.

## USD Physics Compatibility

This crate implements a 1:1 mapping with the [USD Physics Schema](https://openusd.org/release/api/usd_physics_page_front.html).
All components and attributes correspond directly to their USD Physics counterparts,
enabling seamless interchange with USD-based pipelines and tools.

### Bevy Adaptations

The following changes were made to align with Bevy conventions:

| USD Physics | Bevy | Reason |
|-------------|------|--------|
| Angles in degrees | Angles in radians | Bevy and most game engines use radians |
| `prim` terminology | `entity` terminology | Bevy ECS uses entities |
| `stage` terminology | `scene` terminology | Bevy uses scenes |
| `attribute` terminology | `component` terminology | Bevy ECS uses components |
| Relationships via paths | Relationships via `Entity` | Bevy uses entity references |

## Units

Physics uses established concepts of distance and time, plus mass:
- **Distance**: Arbitrary units with [`MetersPerUnit`] for scaling
- **Time**: Seconds
- **Mass**: Arbitrary units with [`KilogramsPerUnit`] for scaling
- **Angles**: Radians (see [`angle`] type alias)

All physical quantities can be decomposed into products of these three basic types.

## Core Concepts

- **scene**: Physics simulation scenes with gravity configuration ([`PhysicsSimulation`], [`GravityDirection`], [`GravityMagnitude`])
- **rigid_body**: Dynamic and kinematic rigid body properties ([`RigidBody`], [`Dynamic`], [`Kinematic`])
- **mass**: Mass, density, center of mass, and inertia configuration ([`Mass`], [`Density`], [`CenterOfMass`])
- **collision**: Collision shape markers for physics interaction ([`CollisionEnabled`])
- **mesh_collision**: Mesh-to-collider approximation methods ([`ColliderFromMeshApproximation`])
- **material**: Friction, restitution, and density material properties ([`DynamicFriction`], [`StaticFriction`], [`Restitution`])
- **collision_group**: Coarse-grained collision filtering groups ([`CollisionGroup`], [`FilteredGroups`])
- **filtered_pairs**: Fine-grained pairwise collision filtering ([`FilteredPairs`])
- **joint**: Joint constraints between rigid bodies ([`PhysicsJoint`], [`RevoluteJoint`], [`PrismaticJoint`], [`SphericalJoint`])
- **articulation**: Reduced coordinate articulated body hierarchies ([`ArticulationRoot`])

## Hierarchy Behavior

When an entity has `RigidBody` applied, all entities in its subtree move rigidly
with it, except descendants that have their own `RigidBody` which form independent
bodies. Collision shapes (`CollisionEnabled`) under a rigid body become part of
that body's collision representation.