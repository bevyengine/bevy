---
title: When & WhenMut System Params
authors: ["@mrchantey"]
pull_requests: [18927]
---

`Res` & `ResMut` now have two alternatives `When` and `WhenMut`, for when a system should skip instead of panic if the resource is not present.

```rust
// existing options
fn panics_if_not_present(res: Res<Foo>){}
fn runs_even_if_not_present(res: Option<Res<Foo>>){}

// new - skip the system if the resource is missing
fn skips_if_not_present(res: When<Foo>){}
```
