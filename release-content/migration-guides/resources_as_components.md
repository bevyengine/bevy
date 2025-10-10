---
title: Resources as Components
pull_requests: [21346]
---

With the introduction of Resources as Components, there are a couple changes.

## `Components::get_valid_resource_id` and `Components::get_resource_id` are deprecated

Because resources are registered using the `TypeId` of `ResourceComponent<SomeResource>`, the behavior of these methods changes.

```rust
// 0.17
world.components().get_resource_id(TypeId::of::<SomeResource>())
world.components().get_valid_resource_id(TypeId::of::<SomeResource>())
// 0.18
world.components().get_resource_id(TypeId::of::<ResourceComponent<SomeResource>>())
world.components().get_valid_resource_id(TypeId::of::<ResourceComponent<SomeResource>>())
```

Since it's confusing to 'get the resource id' when provided a component `TypeId`, these methods are deprecated.
Instead use `Components::get_id` or `Components::get_valid_id`, if you're passing in `TypeId`s and don't forget to get the `TypeId` of the wrapped resource. If instead instead the type is available, use `Components::resource_id` and `Components::valid_resource_id`.

### Important

Non-send resources have not become components. Do not wrap them in `ResourceComponent<_>`.

```rust
// 0.17
world.components().get_resource_id(TypeId::of::<NonSendResource>())
world.components().get_valid_resource_id(TypeId::of::<NonSendResource>())
// 0.18
world.components().get_id(TypeId::of::<NonSendResource>())
world.components().get_valid_id(TypeId::of::<NonSendResource>())
```

## Resources implement `MapEntities` by default

A resource now automatically implements `MapEntities` when using the `#[derive(Resource)]` macro.
To avoid conflicting code, remove `#[derive(MapEntities)]` from every resource.

```rust
// 0.17
#[derive(Resource, MapEntities, Reflect)]
#[reflect(Resource, MapEntities)]
struct SomeResource;

// 0.18
#[derive(Resource, Reflect)]
#[reflect(Resource, MapEntities)]
struct SomeResource;
```

## Resources aren't auto-registered for reflection

<!--This is the one I'd really like to fix before 0.18 -->

In 0.17, auto-registration was introduced to reduce the headache of having to register every single type for runtime reflection.
However, since this feature does not work for generic types like `ResourceComponent<SomeResource>`, resources are currently not auto-registered.

```rust
// add this for each resource you wish to reflect
app.register_type::<ResourceComponent<SomeResource>>()
```

## `clear_entities` doesn't clear all entities

As more and more internal engine concepts become entities, the `clear_entities` becomes more and more destructive.
This is why `World::clear_entities` now only deletes all non-[internal](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/struct.Internal.html) entities. This means that all observers, one-shot-systems, and - most importantly - resources, remain in the world.

`clear_all` does still clear everything.

## Update to scene serialization

Because resources are now entities, we re-use the serialization code for entities for resources.
Which results in a different data layout.

```json
// 0.17
resources: {
  "bevy_scene::serde::tests::MyResource": (
    foo: 123,
  ),
},
entities: {
  4294967293: (
    ...

// 0.18
resources: {
  4294967290: (
    components: {
      "bevy_ecs::resource::ResourceComponent<bevy_scene::serde::tests::MyResource>": ((
        foo: 123,
      )),
    },
  ),
},
entities: {
  4294967293: (
    ...
```

This has the effect that scene files from Bevy 0.17 cannot be read by Bevy 0.18.
