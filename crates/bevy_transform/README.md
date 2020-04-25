# Hierarchical Legion Transform

[![Build Status][build_img]][build_lnk]

[build_img]: https://travis-ci.org/AThilenius/legion_transform.svg?branch=master
[build_lnk]: https://travis-ci.org/AThilenius/legion_transform

A hierarchical space transform system, implemented using [Legion
ECS](https://github.com/TomGillen/legion). The implementation is based heavily
on the new Unity ECS Transformation layout.

## Usage

### TL;DR - Just show me the secret codes and incantations!

See [examples/hierarchy.rs](examples/hierarchy.rs)

```rust
#[allow(unused)]
fn tldr_sample() {
    // Create a normal Legion World
    let mut world = Universe::default().create_world();

    // Create a system bundle (vec of systems) for LegionTransform
    let transform_system_bundle = TransformSystemBundle::default().build();

    let parent_entity = *world
        .insert(
            (),
            vec![(
                // Always needed for an Entity that has any space transform
                LocalToWorld::identity(),
                // The only mutable space transform a parent has is a translation.
                Translation::new(100.0, 0.0, 0.0),
            )],
        )
        .first()
        .unwrap();

    world.insert(
        (),
        vec![
            (
                // Again, always need a `LocalToWorld` component for the Entity to have a custom
                // space transform.
                LocalToWorld::identity(),
                // Here we define a Translation, Rotation and uniform Scale.
                Translation::new(1.0, 2.0, 3.0),
                Rotation::from_euler_angles(3.14, 0.0, 0.0),
                Scale(2.0),
                // Add a Parent and LocalToParent component to attach a child to a parent.
                Parent(parent_entity),
                LocalToParent::identity(),
            );
            4
        ],
    );
}
```

See [examples](/examples) for both transform and hierarchy examples.

### Transform Overview

The Transform and Hierarchy parts of Legion Transform are largely separate and
can thus be explained independently. We will start with space transforms, so for
now completely put hierarchies out of mind (all entities have space transforms
directly from their space to world space).

A 3D space transform can come in many forms. The most generic of these is a
matrix 4x4 which can represent any arbitrary (linear) space transform, including
projections and sheers. These are not rarely useful for entity transformations
though, which are normally defined by things like

- A **Translation** - movement along the X, Y or Z axis.
- A **Rotation** - 3D rotation encoded as a Unit Quaternion to prevent [gimbal
  lock](https://en.wikipedia.org/wiki/Gimbal_lock).
- A **Scale** - Defined as a single floating point values, but often
  **incorrectly defined as a Vector3** (which is a `NonUniformScale`) in other
  engines and 3D applications.
- A **NonUniformScale** - Defined as a scale for the X, Y and Z axis
  independently from each other.

In fact, in Legion Transform, each of the above is it's own `Component` type.
These components can be added in any combination to an `Entity` with the only
exception being that `Scale` and `NonUniformScale` are mutually exclusive.

Higher-order transformations can be built out of combinations of these
components, for example:

- Isometry: `Translation` + `Rotation`
- Similarity: `Translation` + `Rotation` + `Scale`
- Affine: `Translation` + `Rotation` + `NonUniformScale`

The combination of these components will be processed (when they change) by the
`LocalToWorldSystem` which will produce a correct `LocalToWorld` based on the
attached transformations. This `LocalToWorld` is a homogeneous matrix4x4
computed as: `(Translation * (Rotation * (Scale | NonUniformScale)))`.

Breaking apart the transform into separate components means that you need only
pay the runtime cost of computing the actual transform you need per-entity.
Further, having `LocalToWorld` be a separate component means that any static
entity (including those in static hierarchies) can be pre-baked into a
`LocalToWorld` component and the rest of the transform data need not be loaded
or stored in the final build of the game.

In the event that the Entity is a member of a hierarchy, the `LocalToParent`
matrix will house the `(Translation * (Rotation * (Scale | NonUniformScale)))`
computation instead, and the `LocalToWorld` matrix will house the final local
space to world space transformation (after all it's parent transformations have
been computed). In other words, the `LocalToWorld` matrix is **always** the
transformation from an entities local space, directly into world space,
regardless of if the entity is a member of a hierarchy or not.

### Why not just NonUniformScale always?

NonUniformScale is somewhat evil. It has been used (and abused) in countless
game engines and 3D applications. A Transform with a non-uniform scale is known
as an `Affine Transform` and it cannot be applied to things like a sphere
collider in a physics engine without some serious gymnastics, loss of precision
and/or detrimental performance impacts. For this reason, you should always use a
uniform `Scale` component when possible. This component was named `Scale` over
something like "UniformScale" to imply it's status as the default scale
component and `NonUniformScale`'s status as a special case component.

For more info on space transformations, see [nalgebra Points and
Transformations](https://www.nalgebra.org/points_and_transformations/).

### Hierarchies

Hierarchies in Legion Transform are defined in two parts. The first is the
_Source Of Truth_ for the hierarchy, it is always correct and always up-to-date:
the `Parent` Component. This is a component attached to children of a parent (ie
a child 'has a' `Parent`). Users can update this component directly, and because
it points toward the root of the hierarchy tree, it is impossible to form any
other type of graph apart from a tree.

Each time the Legion Transform system bundle is run, the
`LocalToParentPropagateSystem` will also add/modify/remove a `Children`
component on any entity that has children (ie entities that have a `Parent`
component pointing to the parent entity). Because this component is only updated
during the system bundle run, **it can be out of date, incorrect or missing
altogether** after world mutations.

It is important to note that as of today, any member of a hierarchy has it's
`LocalToWorld` matrix re-computed each system bundle run, regardless of
changes. This may someday change, but it is expected that the number of entities
in a dynamic hierarchy for a final game should be small (static hierarchies can
be pre-baked, where each entity gets a pre-baked `LocalToWorld` matrix).

## This is no good 'tall, why didn't you do is <this> way?

The first implementation used Legion `Tags` to store the Parent component for
any child. This allowed for things like `O(1)` lookup of children, but was
deemed way too much fragmentation (Legion is an archetypical, chunked ECS).

The second implementation was based on [this fine article by Michele
Caini](https://skypjack.github.io/2019-06-25-ecs-baf-part-4/) which structures
the hierarchy as explicit parent pointer, a pointer to the first (and only
first) child, and implicitly forms a linked-list of siblings. While elegant, the
actual implementation was both complicated an near-impossible to multi-thread.
For example, iterating through children entities required a global query to the
Legion `World` for each child. I decided a small amount of memory by storing a
possibly-out-of-date `SmallVec` of children was worth sacrificing on parent
entities to make code both simpler and faster (theoretically, I never tested
it).

A lot of other options were considered as well, for example storing the entire
hierarchy out-of-band from the ECS (much like Amethyst pre-Legion does). This
has some pretty nasty drawbacks though. It makes streaming entities much harder,
it means that hierarchies need to be special-case serialized/deserialized with
initialization code being run on the newly deserialized entities. And it means
that the hierarchy does not conform to the rest of the ECS. It also means that
Legion, and all the various optimizations for querying / iterating large numbers
of entities, was going to be mostly unused and a lot of global queries would
need to be made against the `World` while syncing the `World` and out-of-band
data-structure. I felt very strongly against an out-of-band implementation
despite it being simpler to implement upfront.

## Todo

- [ ] Hierarchy maintenance
  - [x] Remove changed `Parent` from `Children` list of the previous parent.
  - [x] Add changed `Parent` to `Children` list of the new parent.
  - [x] Update `PreviousParent` to the new Parent.
  - [x] Handle Entities with removed `Parent` components.
  - [x] Handle Entities with `Children` but without `LocalToWorld` (move their
        children to non-hierarchical).
  - [ ] Handle deleted Legion Entities (requires
        [Legion #13](https://github.com/TomGillen/legion/issues/13))
- [x] Local to world and parent transformation
  - [x] Handle homogeneous `Matrix4<f32>` calculation for combinations of:
    - [x] Translation
    - [x] Rotation
    - [x] Scale
    - [x] NonUniformScale
  - [x] Handle change detection and only recompute `LocalToWorld` when needed.
  - [x] Multi-threaded updates for non-hierarchical `LocalToWorld` computation.
  - [x] Recompute `LocalToParent` each run, always.
- [ ] Transform hierarchy propagation
  - [x] Collect roots of the hierarchy forest
  - [x] Recursively re-compute `LocalToWorld` from the `Parent`'s `LocalToWorld`
        and the `LocalToParent` of each child.
  - [ ] Multi-threaded updates for hierarchical `LocalToWorld` computation.
  - [ ] Compute all changes and flush them to a `CommandBuffer` rather than
        direct mutation of components.

## Blockers

- Legion has no ability to detect deleted entities or components.
  [GitHub Issue #13](https://github.com/TomGillen/legion/issues/13)
