---
title: "`Interned` is now reflectable but requires additional trait bounds"
pull_requests: [22472]
---

`Interned<T>` now requires all instances with `T` to implement `Internable`, where
previously only the `PartialEq`, `Eq`, and `Hash` implementations requried it.
Implement `Internable` for `T` to fix this.
