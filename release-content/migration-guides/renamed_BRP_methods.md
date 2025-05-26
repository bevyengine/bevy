---
title: Renamed BRP methods to be more hierarchical
pull_requests: [19377]
---

Most Bevy Remote Protocol methods have been renamed to be more hierarchical.
The word `destroy` has also been replaced with `despawn` to match the rest of the engine.

| Old                    | New                                |
|------------------------|------------------------------------|
| `bevy/query`           | `bevy/world/query`                 |
| `bevy/spawn`           | `bevy/world/spawn`                 |
| `bevy/destroy`         | `bevy/world/despawn`               |
| `bevy/reparent`        | `bevy/world/reparent`              |
| `bevy/get`             | `bevy/world/components/get`        |
| `bevy/insert`          | `bevy/world/components/insert`     |
| `bevy/remove`          | `bevy/world/components/remove`     |
| `bevy/list`            | `bevy/world/components/list`       |
| `bevy/mutate`          | `bevy/world/components/mutate`     |
| `bevy/get+watch`       | `bevy/world/components/get+watch`  |
| `bevy/list+watch`      | `bevy/world/components/list+watch` |
| `bevy/get_resource`    | `bevy/world/resources/get`         |
| `bevy/insert_resource` | `bevy/world/resources/insert`      |
| `bevy/remove_resource` | `bevy/world/resources/remove`      |
| `bevy/list_resources`  | `bevy/world/resources/list`        |
| `bevy/mutate_resource` | `bevy/world/resources/mutate`      |
