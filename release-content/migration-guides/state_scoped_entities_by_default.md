---
title: Entities are now state scoped by default
pull_requests: [19354]
---

State scoped entities is now enabled by default, and you don't need to call `app.enable_state_scoped_entities::<State>()` anymore.

If you were previously adding the `#[states(scoped_entities)]` attribute when deriving the `States` trait, you can remove it.

If you want to keep the previous behavior, you must add the attribute `#[states(scoped_entities = false)]`.
