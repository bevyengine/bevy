---
title: Query Interfaces
pull_requests: [22670]
---

The function `WorldQuery::init_state` now takes `&World` instead of `&mut World`.
Callers have no change here.
For implementers, you are no longer allowed to mutate the world during query registration.
If you were mutating it, consider instead doing some manual registration work before creating the query.
While this is annoying for some cases, query states were never intended to be a way to change the state of a world.
Prefer explicit world mutations.

The `try_query` interface, including `QueryState::try_new`, `World::try_query`, and `World::try_query_filtered` have all been deprecated.
These can be done infallibility through `QueryState::new`, `World::query`, and `World::query_filtered`, which now only require `&World`.
