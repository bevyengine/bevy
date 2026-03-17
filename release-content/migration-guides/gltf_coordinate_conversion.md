---
title: Improved glTF coordinate conversion
pull_requests: [23353]
---

glTF coordinate conversion has been changed to add new options and fix
inconsistencies.

```rust
struct GltfConvertCoordinates {
    rotate_scenes: bool, // Changed in 0.19
    rotate_nodes: bool, // New in 0.19
    rotate_meshes: bool,
    semantics: GltfConvertSemantics, // New in 0.19
}
```

*CAUTION: Coordinate conversion is an experimental feature - behavior may change
in future versions.*

The goal of coordinate conversion is to take objects that face forward in the
glTF and change them to match the direction of Bevy's `Transform::forward`.
Conversion can be necessary because glTF's
[standard coordinate system semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
are "+Z forward", while Bevy's are "-Z forward" (although not all glTF files
follow the standard).

Coordinate conversion remains disabled by default - if you haven't enabled
it then your glTFs will work the same as before.

## Node Conversion

`GltfConvertCoordinates` has a new `rotate_nodes` option. Enabling this option
will convert the coordinates of entities that correspond to nodes in the glTF
scene.

```rust
settings.convert_coordinates.rotate_nodes = true;
```

For convenience, there's a new `GltfConvertCoordinates::ALL` constant that
enables scene, node and mesh conversion.

(Note that node conversion was present in Bevy 0.17, but removed in 0.18. The
feature has been restored in 0.19.)

## Scene Conversion

The `GltfConvertCoordinates::rotate_scene_entities` option has been renamed to
`rotate_scenes`, and its behavior has been changed to fix inconsistencies.

When a glTF is spawned as a Bevy scene, its entity hierarchy usually looks like
this:

- User entity
  - glTF scene root entity
    - glTF root node entities

"User entity" is the entity that was spawned with a `SceneRoot` component or
passed to `SceneSpawner::spawn_as_child`.

In Bevy 0.18, `rotate_scene_entities` would rotate the glTF scene root entity.

- User entity.
  - glTF scene root entity \<--- ROTATED
    - glTF root node entities

This gave the correct visual result, but left the glTF scene root entity with
incorrect semantics - its `Transform::forward` would be wrong.

In Bevy 0.19, the option has been renamed to `rotate_scenes` and its behavior
has changed - it now rotates the glTF root node entities.

- User entity.
  - glTF scene root entity
    - glTF root node entities \<--- ROTATED

Now the glTF scene root entity has the correct semantics, while the visual
result stays the same.

## Arbitrary Semantics

`GltfConvertCoordinates` has a new `semantics` option for arbitrary semantic
conversion. 

Some glTF files don't follow the standard "+Z forward" semantics. The example
below shows how a glTF file with "+X forward, +Y up" semantics can be converted
to Bevy's semantics.

```rust
settings.convert_coordinates = Some(GltfConvertCoordinates::ALL.with_semantics(
    GltfConvertSemantics::All(SemanticsConversion {
        source: Semantics {
            forward: SignedAxis::X,
            up: SignedAxis::Y,
        },
        target: Semantics::BEVY,
    })
));
```

