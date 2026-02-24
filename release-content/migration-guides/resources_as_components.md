---
title: Resources as Components
pull_requests: [20934]
---

## `#[derive(Resource)]` implements the `Component` trait

In 0.19, `Resource` is a subtrait of `Component` and `#[derive(Resource)]` implements both `Resource` as well as `Component`.
This means it's no longer possible to doubly derive both `Component` and `Resource`.
Instead, you should split them up:

```rust
// 0.18.0
#[derive(Component, Resource)]
struct Dual
```

becomes

```rust
// 0.19.0
#[derive(Component)]
struct DualComp;

#[derive(Resource)]
struct DualRes;
```

Consequently, `UiDebugOverlay` is split into `GlobalUiDebugOverlay` (resource) and `UiDebugOverlay` (component), and `UiDebugOptions` is split into `GlobalUiDebugOptions` (resource) and `UiDebugOptions` (component).

## `#[reflect(Resource)]` Changes

The `ReflectResource` is a ZST (zero-sized type) in 0.19 and only functions to signify that the trait is reflected.
Instead, `#[reflect(Resource)]` also reflects the `Component` trait, so use `ReflectComponent` instead.
This is likely to show up in code that uses reflection, like BRP (Bevy Reflect Protocol) and `bevy_scene`.

## Renaming Non-Send Resources to Non-Send Data

Previously there were two types of resources: `Send` resources and `!Send` resources.
Now that `Send` resources are stored as components, `!Send` resources have little in common with their `Send` counterparts.
This is why non-send resources are being renamed to non-send data.
The following APIs are effected:

- `App::init_non_send_resource` is deprecated in favor of `App::init_non_send`.
- `App::insert_non_send_resource` is deprecated in favor of `App::insert_non_send`.
- `DeferredWorld::non_send_resource_mut` is deprecated in favor of `DeferredWorld::non_send_mut`.
- `DeferredWorld::get_non_send_resource_mut` is deprecated in favor of `DeferredWorld::get_non_send_mut`.
- `ResourceData<SEND: true>` is removed, while `ResourceData<SEND: false>` is renamed to `NonSendData`.
- `Resources<SEND: true>` is removed and `Resources<Send: false>` is renamed to `NonSends`.
- `UnsafeWorldCell::get_non_send_resource` is deprecated in favor of `UnsafeWorldCell::get_non_send`.
- `UnsafeWorldCell::get_non_send_resource_by_id` is deprecated in favor of `UnsafeWorldCell::get_non_send_by_id`.
- `UnsafeWorldCell::get_non_send_resource_mut` is deprecated in favor of `UnsafeWorldCell::get_non_send_mut`.
- `UnsafeWorldCell::get_non_send_resource_mut_by_id` is deprecated in favor of `UnsafeWorldCell::get_non_send_mut_by_id`.
- `World::init_non_send_resource` is deprecated in favor of `World::init_non_send`.
- `World::insert_non_send_resource` is deprecated in favor of `World::insert_non_send`.
- `World::remove_non_send_resource` is deprecated in favor of `World::remove_non_send`.
- `World::non_send_resource` is deprecated in favor of`World::non_send`.
- `World::non_send_resource_mut` is deprecated in favor of `World::non_send_mut`.
- `World::get_non_send_resource` is deprecated in favor of `World::get_non_send`.
- `World::get_non_send_resource_mut` is deprecated in favor of `World::get_non_send_mut`.

## Component Registration

Before using components and resources they must be registered to a world.
The registration process for components and resources is very similar and now that `Send` resources *are* components, we're able to simplify some of the code; removing / deprecating some methods.

- `Components::register_resource_unchecked` is renamed to `Components::register_non_send_unchecked`.
- `Components::get_valid_resource_id` was deprecated in favor of `Components::get_valid_id`.
- `Components::valid_resource_id` was deprecated in favor of `Components::valid_component_id`.
- `Components::resource_id` was deprecated in favor of `Components::component_id`.
- `ComponentsRegistrator::register_resource` is deprecated in favor of `ComponentsRegistrator::register_component`.
- `ComponentsRegistrator::register_resource_with` is renamed to `ComponentsRegistrator::register_non_send_with`.
- `ComponentsRegistrator::register_resource_with_descriptor` is removed in favor of `ComponentsRegistrator::register_component_with_descriptor`.
- `ComponentsQueuedRegistrator::queue_register_resource_with_descriptor` was removed in favor of `ComponentsQueuedRegistrator::queue_register_component_with_descriptor`.
- `ComponentsQueuedRegistrator::queue_register_resource` was deprecated in favor of `ComponentsQueuedRegistrator::queue_register_component`.
- `ComponentDescriptor::new_resource` was deprecated in favor of `ComponentDescriptor::new`
- `ComponentDescriptor::new_resource` was deprecated in favor of `ComponentDescriptor::new`
- `World::register_resource_with_descriptor was renamed to World::register_non_send_with_descriptor`.

## Miscellaneous

Since `MapEntities` is implemented by default for components, it's no longer necessary to add `derive(MapEntities)` to a resource.

```rust
// 0.17.0
#[derive(Resource, MapEntities)]
struct EntityStruct(#[entities] Entity);

// 0.18.0
#[derive(Resource)]
struct EntityStruct(#[entities] Entity);
```

Next, `World::clear_entities` now also clears all resources, and `World::clear_all` now clears all entities, resources, and non-send data.

Lastly, `World::remove_resource_by_id` now returns `bool` instead of `Option<()>`.
