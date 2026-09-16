---
title: "Component ID constants are more type safe"
pull_requests: [25779]
---

`ComponentId` constants like `ADD`, `INSERT`, `DISCARD`, `REMOVE`, `DESPAWN`,
and `IS_RESOURCE` are now `ComponentId` constants rather than `usize`, to
improve type safety. Use `ComponentId::index()` if you need the underlying `usize`
value.
