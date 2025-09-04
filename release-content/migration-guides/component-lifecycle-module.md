---
title: Component lifecycle reorganization
pull_requests: [19543]
---

To improve documentation, discoverability and internal organization, we've gathered all of the component lifecycle-related code we could and moved it into a dedicated `lifecycle` module.

The lifecycle / observer types (`Add`, `Insert`, `Remove`, `Replace`, `Despawn`) have been moved from the `bevy_ecs::world` to `bevy_ecs::lifecycle`.

The same move has been done for the more internal (but public) `ComponentId` constants: `ADD`, `INSERT`, `REMOVE`, `REPLACE`, `DESPAWN`.

The code for hooks (`HookContext`, `ComponentHook`, `ComponentHooks`) has been extracted from the very long `bevy_ecs::components` module, and now lives in the `bevy_ecs::lifecycle` module.

The `RemovedComponents` `SystemParam`, along with the public `RemovedIter`, `RemovedIterWithId` and `RemovedComponentEvents` have also been moved into this module as they serve a similar role. All references to `bevy_ecs::removal_detection` can be replaced with `bevy_ecs::lifecycle`.
