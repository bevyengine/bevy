---
title: `SceneSpawner` methods have been renamed and replaced.
pull_requests: [18358]
---

Some methods on `SceneSpawner` have been renamed:
    - `despawn` -> `despawn_dynamic`
    - `despawn_sync` -> `despawn_dynamic_sync`
    - `update_spawned_scenes` -> `update_spawned_dynamic_scenes`

In their place, we've added `despawn`, `despawn_sync`, and `update_spawned_scenes` which all act on
`Scene`s (as opposed to `DynamicScene`s).
