---
title: Component Constants reorganization
pull_requests: [TODO]
---

The lifecycle / observer types (`OnAdd`, `OnInsert`, `OnRemove`, `OnReplace`, `OnDespawn`) have been moved from the `bevy_ecs::world` to `bevy_ecs::component_lifecycle`.

The same move has been done for the more internal (but public) `ComponentId` constants: `ON_ADD`, `ON_INSERT`, `ON_REMOVE`, `ON_REPLACE`, `ON_DESPAWN`.
