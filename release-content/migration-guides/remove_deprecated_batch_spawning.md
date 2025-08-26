---
title: Removed Deprecated Batch Spawning Methods
pull_requests: [18148]
---

The following deprecated functions have been removed:

- `Commands::insert_or_spawn_batch`
- `World::insert_or_spawn_batch`
- `World::insert_or_spawn_batch_with_caller`

These functions, when used incorrectly, could cause major performance problems and were generally viewed as anti-patterns and foot guns.
They were deprecated in 0.16 for being unnecessary with the retained render world and easily misused.

Instead of these functions consider doing one of the following:

Option A) Instead of despawning entities, insert the `Disabled` component, and instead of respawning them at particular ids, use `try_insert_batch` or `insert_batch` and remove `Disabled`.

Option B) Instead of giving special meaning to an entity id, simply use `spawn_batch` and ensure entity references are valid when despawning.
