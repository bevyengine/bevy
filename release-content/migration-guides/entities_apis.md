---
title: Entities APIs
pull_requests: [19350, 19433, 19451]
---

In 0.16, entities could have zero or more components.
In 0.17, a new state for an entity is introduced: null/not constructed.
Entities can now be constructed and destructed within their spawned life cycle.
This opens up a lot of room for performance improvement, but caused a lot of breaking changes:

### `Entities` rework

A lot has changed here.
First, `alloc`, `free`, `reserve`, `reserve_entity`, `reserve_entities`, `flush`, `flush_as_invalid`, `total_count`, `used_count`, and `total_prospective_count` have all been removed.
Allocation and freeing have been made private, but there are new ways to accomplish this.
Reservation and flushing have been completely removed as they are no longer needed.
Instead of reserving an entity and later flushing it, you can `World::spawn_null` (which does not need mutable access), and `World::construct` can be used to "flush" it.
The counting methods have been reworked in the absence of flushing:
`len` and `is_empty` now deal with how many entity rows have been allocated (not necessarily the number that have been constructed/spawned),
and the new `count_constructed` and `any_constructed` are similar to the old `len` and `is_empty` behavior.
In terms of getting information from `Entities`, `get` and `contains` has been reworked to include non-constructed entities.
If you only want constructed entities, `get_constructed` and `contains_constructed` are available.
Additionally, `get` now returns `Result<EntityIdLocation, EntityDoesNotExistError>` instead of `Option<EntityLocation>` for clarity.
`EntityIdLocation` is an alias for `Option<EntityLocation>`, as entities now may or may not have a location, depending on if it is constructed or not.

`EntityDoesNotExistError::location` has been replaced by `EntityDoesNotExistError::generation` of type `EntityGeneration`.
This is because a not constructed entity is now still considered to exist.
The only way an `Entity` can not exist is if it has the wrong generation; the right one is now in `EntityDoesNotExistError::generation`.
If you only want constructed entities, the new `ConstructedEntityDoesNotExistError` is available.

As an alternative to `free`, simply create a `EntityWorldMut` (if it fails, it's already been freed), and despawn it.
If the entity was already destructed, despawning will just free the entity internally.

### Entity Pointers

All entity pointers location information has changed from `EntityLocation` to `EntityIdLocation`, as they can now point to non-constructed entities.
This extends to archetypes too, many `.archetype` methods now return `Option<&Archetype>`.
Notably, `EntityWorldMut` continues to have panicking methods which assume the entity is constructed.

### Entity Commands

`Commands::new_from_entities` now also needs `&EntitiesAllocator`, which can be obtained from `UnsafeWorldCell::entities_allocator`.
`Commands::get_entity` does not error for non-constructed entities.
If you only want constructed entities, use `Commands::get_constructed_entity`

### Other entity interactions

`BundleSpawner::spawn_non_existent` is now `BundleSpawner::construct`.
`World::inspect_entity` now errors with `ConstructedEntityDoesNotExistError` instead of `EntityDoesNotExistError`.
`QueryEntityError::EntityDoesNotExist` now wraps `ConstructedEntityDoesNotExistError`.
`EntityDespawnError` has been renamed to `EntityDestructError`.
`ComputeGlobalTransformError::NoSuchEntity` and `ComputeGlobalTransformError::MalformedHierarchy` now wrap `ConstructedEntityDoesNotExistError`.
