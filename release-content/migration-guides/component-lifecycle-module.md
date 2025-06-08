---
title: Component lifecycle reorganization
pull_requests: [TODO]
---

To improve documentation, discoverability and internal organization, we've gathered all of the component lifecycle-related code we could and moved it into a dedicated `component_lifecycle` module.

The lifecycle / observer types (`OnAdd`, `OnInsert`, `OnRemove`, `OnReplace`, `OnDespawn`) have been moved from the `bevy_ecs::world` to `bevy_ecs::component_lifecycle`.

The same move has been done for the more internal (but public) `ComponentId` constants: `ON_ADD`, `ON_INSERT`, `ON_REMOVE`, `ON_REPLACE`, `ON_DESPAWN`.

The code for hooks (`HookContext`, `ComponentHook`, `ComponentHooks`) has been extracted from the very long `bevy_ecs::components` module, and now lives in the `bevy_ecs::components` module.
