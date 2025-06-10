---
title: Renamed BRP methods
pull_requests: [19377]
---

Most Bevy Remote Protocol methods have been renamed to be more explicit.
The word `destroy` has also been replaced with `despawn` to match the rest of the engine.

| Old                    | New                           |
|------------------------|-------------------------------|
| `bevy/query`           | `world.query`                 |
| `bevy/spawn`           | `world.spawn_entity`          |
| `bevy/destroy`         | `world.despawn_entity`        |
| `bevy/reparent`        | `world.reparent_entities`     |
| `bevy/get`             | `world.get_components`        |
| `bevy/insert`          | `world.insert_components`     |
| `bevy/remove`          | `world.remove_components`     |
| `bevy/list`            | `world.list_components`       |
| `bevy/mutate`          | `world.mutate_components`     |
| `bevy/get+watch`       | `world.get_components+watch`  |
| `bevy/list+watch`      | `world.list_components+watch` |
| `bevy/get_resource`    | `world.get_resources`         |
| `bevy/insert_resource` | `world.insert_resources`      |
| `bevy/remove_resource` | `world.remove_resources`      |
| `bevy/list_resources`  | `world.list_resources`        |
| `bevy/mutate_resource` | `world.mutate_resources`      |
| `registry/schema`      | `registry.schema`             |
