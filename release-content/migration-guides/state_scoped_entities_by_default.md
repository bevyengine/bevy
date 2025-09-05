---
title: State-scoped entities are now always enabled implicitly
pull_requests: [19354, 20883]
---

State scoped entities is now always enabled by default, and as a consequence, `app.enable_state_scoped_entities::<State>()` is no longer needed.
It has been marked as deprecated and does nothing when called.

The attribute `#[states(scoped_entities)]` has been removed.
