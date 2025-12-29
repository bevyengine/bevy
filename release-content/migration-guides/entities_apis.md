---
title: Entities APIs
pull_requests: [19350, 19433, 19451]
---

Entities are spawned by allocating their id and then giving that id a location within the world.
In 0.17, this was done in one stroke through `spawn` and `Entities::flush`.
In 0.18, the flushing functionality has been removed in favor of `spawn`ing individual `EntityRow`s instead.
Don't worry, these changes don't affect the common operations like `spawn` and `despawn`, but the did impact the peripheral interfaces and error types.
For a full explanation of the new entity paradigm, errors and terms, see the new `entity` module docs.
If you want more background for the justification of these changes or more information about where these new terms come from, see pr #19451.
This opens up a lot of room for performance improvement but also caused a lot of breaking changes:

## `Entities` rework

A lot has changed here.
First, `alloc`, `free`, `reserve`, `reserve_entity`, `reserve_entities`, `flush`, `flush_as_invalid`, `EntityDoesNotExistError`, `total_count`, `used_count`, and `total_prospective_count` have all been removed 😱.

Allocation has moved to the new `EntitiesAllocator` type, accessible via `World::entities_allocator` and `World::entities_allocator_mut`, which have `alloc`, `free`, and `alloc_many`.

Reservation and flushing have been completely removed as they are no longer needed.
Instead of reserving an entity and later flushing it, you can `EntitiesAllocator::alloc` (which does not need mutable access), and `World::spawn_at` can be used to "flush" the entity.

The counting methods have been reworked in the absence of flushing:
`len` and `is_empty` now deal with how many entity rows have been allocated (not necessarily the number that have been spawned),
and the new `count_spawned` and `any_spawned` are similar to the old `len` and `is_empty` behavior but are now O(n).

In terms of getting information from `Entities`, `get` and `contains` has been reworked to include non-spawned entities.
If you only want spawned entities, `get_spawned` and `contains_spawned` are available.
Additionally, `get` now returns `Result<Option<EntityLocation>, InvalidEntityError>` instead of `Option<EntityLocation>` for clarity.
Entities now may or may not have a location, depending on if it is spawned or not.

`EntityDoesNotExistError` has been removed and reworked.
See the new entity module docs for more, but:
When an entity's generation is not up to date with its row, `InvalidEntityError` is produced.
When an entity index's `Option<EntityLocation>` is `None`, `EntityValidButNotSpawnedError` is produced.
When an `Entity` is expected to be spawned but is not (either because its generation is outdated or because its row is not spawned), `EntityNotSpawnedError` is produced.
A few other wrapper error types have slightly changed as well, generally moving from "entity does not exist" to "entity is not spawned".

### Entity Ids

Entity ids previously used "row" terminology, but now use "index" terminology as that more closely specifies its current implementation.
As such, all functions and types dealing with the previous `EntityRow` have had their names 1 to 1 mapped to index.
Ex: `EntityRow` -> `EntityIndex`, `Entity::row` -> `Entity::index`, `Entity::from_row` -> `Entity::from_index`, etc.
Note that `Entity::index` did exist before. It served to give the numeric representation of the `EntityRow`.
The same functionality exists, now under `Entity::index_u32`.

### Entity Pointers

When migrating, entity pointers, like `EntityRef`, were changed to assume that the entity they point to is spawned.
This was not necessarily checked before, so the errors for creating an entity pointer is now `EntityNotSpawnedError`.
This probably will not affect you since creating a pointer to an entity that was not spawned is kinda pointless.

It is still possible to invalidate an `EntityWorldMut` by despawning it from commands. (Ex: The hook for adding a component to an entity actually despawns the entity it was added to.)
If that happens, it may lead to panics, but `EntityWorldMut::is_spawned` has been added to help detect that.

### Entity Commands

`Commands::new_from_entities` now also needs `&EntitiesAllocator`, which can be obtained from `UnsafeWorldCell::entities_allocator`.
`Commands::get_entity` does not error for non-spawned entities, making it useful to amend an entity you have queued to spawn through commands.
If you only want spawned entities, use `Commands::get_spawned_entity`.

### Other entity interactions

The `ArchetypeRow::INVALID` and `ArchetypeId::INVALID` constants have been removed, since they are no longer needed for flushing.
If you depended on these, use options instead.

`BundleSpawner::spawn_non_existent` is now `BundleSpawner::construct`.
`World::inspect_entity` now errors with `EntityNotSpawnedError` instead of `EntityDoesNotExistError`.
`QueryEntityError::EntityDoesNotExist` is now `QueryEntityError::NotSpawned`.
`ComputeGlobalTransformError::NoSuchEntity` and `ComputeGlobalTransformError::MalformedHierarchy` now wrap `EntityNotSpawnedError`.
