---
title: Deprecate `iter_entities` and `iter_entities_mut`.
pull_requests: [20260]
---

In Bevy 0.17.0 we deprecate `world.iter_entities()` and `world.iter_entities_mut()`.
Use `world.query::<EntityMut>().iter(&world)` and `world.query::<EntityRef>().iter(&mut world)` instead.

This may not return every single entity, because of [default filter queries](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/index.html). If you really intend to query disabled entities too, consider removing the `DefaultQueryFilters` resource from the world before querying the elements. You can also add an `Allow<Component>` filter to allow a specific disabled `Component` to show up in the query.
