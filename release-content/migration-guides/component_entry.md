---
title: "`Entry` enum is now `ComponentEntry`"
pull_requests: [19517]
---

The `Entry` enum in `bevy::ecs::world` has been renamed to `ComponentEntry`, to avoid name clashes with `hash_map`, `hash_table` and `hash_set` `Entry` types.

Correspondingly, the nested `OccupiedEntry` and `VacantEntry` enums have been renamed to `OccupiedComponentEntry` and `VacantComponentEntry`.
