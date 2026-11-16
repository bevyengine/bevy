---
title: Enums deriving `Reflect` must also implement `FromReflect`
pull_requests: [26222]
---

Enums that derive `Reflect` are now **required** to implement `FromReflect`. By default, deriving
`Reflect` also derives `FromReflect`. This can be disabled with `#[reflect(from_reflect = false)]`.
Enums that have already disabled this must now opt back into deriving `FromReflect` or implement it manually.
