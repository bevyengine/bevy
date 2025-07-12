---
title: Resources as Components
pull_requests: [19711]
---

Resources are very similair to Components: they are both data that can be stored in the ECS and queried.
The only real difference between them is that querying a resource will return either one or zero resources, whereas querying for a component can return any number of entities that match it.

Even so, resources and components have always been seperate concepts within the ECS.
This leads to some annoying restrictions.
While components have [`ComponentHooks`](https://docs.rs/bevy/latest/bevy/ecs/component/struct.ComponentHooks.html), it's not possible to add lifecycle hooks to resources.
Moreover, the engine internals contain a lot of duplication because of it.

This motivates us to transition resources to components, and while most of the public API will stay the same, some breaking changes are inevitable.

This PR adds a dummy entity alongside every resource. This entity is inserted and removed alongside resources and doesn't do anything (yet).

This changes `World::entities().len()` as there are more entities than you might expect there to be. For example, a new world, no longer contains zero entities. This is mostly important for unit tests.

Two methods have been added `World::entity_count()` and `World::resource_count()`. The former returns the number of entities without the resource entities, while the latter returns the number of resources in the world.

While the marker component `IsResource` is added to [default query filters](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/struct.DefaultQueryFilters.html), during world creation, resource entities might still show up in broad queries with [`EntityMutExcept`](https://docs.rs/bevy/latest/bevy/ecs/world/struct.EntityMutExcept.html) and [`EntityRefExcept`](https://docs.rs/bevy/latest/bevy/ecs/world/struct.EntityRefExcept.html).
They also show up in `World::iter_entities()`, `World::iter_entities_mut()` and [`QueryBuilder`](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.QueryBuilder.html).

Lastly, because of the entity bump, the input and output of the `bevy_scene` crate is not equivalent to the previous version, meaning that it's unadvisable to read in scenes from the previous version into the current one.
