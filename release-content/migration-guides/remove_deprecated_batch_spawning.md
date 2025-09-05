---
title: Removed Deprecated Batch Spawning Methods
pull_requests: [18148]
---

The following deprecated functions have been removed:

- `Commands::insert_or_spawn_batch`
- `World::insert_or_spawn_batch`
- `World::insert_or_spawn_batch_with_caller`

These functions, when used incorrectly, could cause major performance problems and generally violated the privacy of the ECS internals in ways
that the Bevy maintainers were not prepared to support long-term.
They were deprecated in 0.16 due to their potential for misuse, as the retained render world removed Bevy's own uses of these methods.

Instead of allocating entities with specific identifiers, consider one of the following:

1. Instead of despawning entities, insert the `Disabled` component, and instead of respawning them at particular ids, use `try_insert_batch` or `insert_batch` and remove `Disabled`.

2. Instead of giving special meaning to an entity id, simply use `spawn_batch` and ensure entity references are valid when despawning.

3. Use your own stable identifier and a map to `Entity` identifiers, with the help of the `EntityMapper` trait.
