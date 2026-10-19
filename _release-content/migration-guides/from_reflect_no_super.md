---
title: FromReflect is now a super trait of Reflect.
pull_requests: [26222, 26249]
---

Previously, `Reflect` was a super trait of `FromReflect`. Now, `FromReflect` is a super trait of
`Reflect`. This means trait bounds that used to require `T: FromReflect` may instead need to require
`T: Reflect`.

In some cases however, this changed trait bound is **not** needed. Check if you actually depend
on any `Reflect` methods; `FromReflect` alone may be enough!

In addition, this means that **all** `Reflect` types now need to implement `FromReflect`. By
default, `#[derive(Reflect)]` will also derive `FromReflect`. However, users may disable this with
`#[reflect(from_reflect = false)]`. For these cases, you now **must** implement `FromReflect` (where
previously you could choose not to). If your type truly cannot be created through reflection, you
can implement the trait as:

```rust
impl FromReflect for MyStruct {
  fn from_reflect(_reflect: &dyn PartialReflect) -> Option<Self> {
    None
  }
}
```
