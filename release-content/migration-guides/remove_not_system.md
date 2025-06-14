---
title: Remove `NotSystem`
pull_requests: [19580]
---

Not used anywhere in the engine and very niche to users, `NotSystem` has been removed.

If you were using it, consider redefining it yourself as in this [example](https://docs.rs/bevy/0.16.1/bevy/ecs/system/trait.Adapt.html#examples).
