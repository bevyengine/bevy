---
title: Internal Entities
pull_requests: [20204]
---

Bevy 0.17 introduces internal entities. Entities tagged by the `Internal` component that are hidden from most queries using [`DefaultQueryFilters`](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/index.html).

Currently, both [`Observer`s](https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html) and systems that are registered through [`World::register_system`](https://docs.rs/bevy/latest/bevy/prelude/struct.World.html#method.register_system) are considered internal entities.

If you queried them before, add the `Allow<Internal>` filter to the query to bypass the default filter.
