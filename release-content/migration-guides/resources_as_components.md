---
title: Resources as Components
pull_requests: [19711]
---

Resources are very similar to Components: they are both data that can be stored in the ECS and queried.
The only real difference between them is that querying a resource will return either one or zero resources, whereas querying for a component can return any number of matching entities.

Even so, resources and components have always been separate concepts within the ECS.
This leads to some annoying restrictions.
While components have [`ComponentHooks`](https://docs.rs/bevy/latest/bevy/ecs/component/struct.ComponentHooks.html), it's not possible to add lifecycle hooks to resources.
Moreover, the engine internals contain a lot of duplication because of it.

This motivates us to transition resources to components, and while most of the public API will stay the same, some breaking changes are inevitable.

This PR adds a dummy entity alongside every resource.
This entity is inserted and removed alongside resources and doesn't do anything (yet).

This changes `World::entities().len()` as there are more entities than you might expect there to be.
For example, a new world, no longer contains zero entities.
This is mostly important for unit tests.
If there is any place you are currently using `world.entities().len()`, we recommend you instead use a query `world.query<RelevantComponent>().query(&world).count()`.

Meanwhile, resource entities are also tagged with `IsResource` and `Internal` marker components.
For more information, checkout the migration guide for internal entities.
In summary, internal entities are added to [default query filters](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/struct.DefaultQueryFilters.html), and will not show up in most queries.

Lastly, because of the entity bump, the input and output of the `bevy_scene` crate is not equivalent to the previous version, meaning that it's unadvisable to read in scenes from the previous version into the current one.
