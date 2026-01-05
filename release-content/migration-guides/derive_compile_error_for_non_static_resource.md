---
title: Derive on Resource will fail when using non-static lifetimes
pull_requests: [21385]
---

Any type with `#[derive(Resource)]` that uses non-static lifetime will no longer compile.

```rust
// Will no longer compile in 0.18, `'a` should be `'static`.
#[derive(Resource)]
struct Foo<'a> {
   bar: &'a str
}
```
