---
title: Manual Entity Creation and Representation
pull_requests: [18704, 19121]
---

An entity is made of two parts: and index and a generation. Both have changes:

### Index

`Entity` no longer stores its index as a plain `u32` but as the new `EntityRow`, which wraps a `NonMaxU32`.
Previously, `Entity::index` could be `u32::MAX`, but that is no longer a valid index.
As a result, `Entity::from_raw` now takes `EntityRow` as a parameter instead of `u32`. `EntityRow` can be constructed via `EntityRow::new`, which takes a `NonMaxU32`.
If you don't want to add [nonmax](https://docs.rs/nonmax/latest/nonmax/) as a dependency, use `Entity::from_raw_u32` which is identical to the previous `Entity::from_raw`, except that it now returns `Option` where the result is `None` if `u32::MAX` is passed.

Bevy made this change because it puts a niche in the `EntityRow` type which makes `Option<EntityRow>` half the size of `Option<u32>`.
This is used internally to open up performance improvements to the ECS.

Although you probably shouldn't be making entities manually, it is sometimes useful to do so for tests.
To migrate tests, use:

```diff
- let entity = Entity::from_raw(1);
+ let entity = Entity::from_raw_u32(1).unwrap();
```

If you are creating entities manually in production, don't do that!
Use `Entities::alloc` instead.
But if you must create one manually, either reuse a `EntityRow` you know to be valid by using `Entity::from_raw` and `Entity::row`, or handle the error case of `None` returning from `Entity::from_raw_u32(my_index)`.

### Generation

An entity's generation is no longer a `NonZeroU32`.
Instead, it is an `EntityGeneration`.
Internally, this stores a `u32`, but that might change later.

Working with the generation directly has never been recommended, but it is sometimes useful to do so in tests.
To create a generation do `EntityGeneration::FIRST.after_versions(expected_generation)`.
To use this in tests, do `assert_eq!(entity.generation(), EntityGeneration::FIRST.after_versions(expected_generation))`.

### Removed Interfaces

The `identifier` module and all its contents have been removed.
These features have been slimmed down and rolled into `Entity`.

This means that where `Result<T, IdentifierError>` was returned, `Option<T>` is now returned.

### Functionality

It is well documented that both the bit format, serialization, and `Ord` implementations for `Entity` are subject to change between versions.
Those have all changed in this version.

For entity ordering, the order still prioritizes an entity's generation, but after that, it now considers higher index entities less than lower index entities.

The changes to serialization and the bit format are directly related.
Effectively, this means that all serialized and transmuted entities will not work as expected and may crash.
To migrate, invert the lower 32 bits of the 64 representation of the entity, and subtract 1 from the upper bits.
Again, this is still subject to change, and serialized scenes may break between versions.

### Length Representation

Because the maximum index of an entity is now `NonZeroU32::MAX`, the maximum number of entities (and length of unique entity row collections) is `u32::MAX`.
As a result, a lot of APIs that returned `usize` have been changed to `u32`.

These include:

- `Archetype::len`
- `Table::entity_count`

### Other kinds of entity rows

Since the `EntityRow` is a `NonMaxU32`, `TableRow` and `ArchetypeRow` have been given the same treatment.
They now wrap a `NonMaxU32`, allowing more performance optimizations.

Additionally, they have been given new, standardized interfaces:

- `fn new(NonMaxU32)`
- `fn index(self) -> usize`
- `fn index_u32(self) -> u32`

The other interfaces for these types have been removed.
Although it's not usually recommended to be creating these types manually, if you run into any issues migrating here, please open an issue.
If all else fails, `TableRow` and `ArchetypeRow` are `repr(transparent)`, allowing careful transmutations.
