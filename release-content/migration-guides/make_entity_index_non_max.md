---
title: Manual Entity Creation
pull_requests: [18704]
---

`Entity` no longer stores its index as a plain `u32` but as the new `EntityRow`, which wraps a `NonMaxU32`. Previously, `Entity::index` could be `u32::MAX`, but that is no longer a valid index. As a result, `Entity::from_raw` now takes `EntityRow` as a parameter instead of `u32`. `EntityRow` can be constructed via `EntityRow::new`, which takes a `NonMaxU32`. If you don't want to add [nonmax](https://docs.rs/nonmax/latest/nonmax/) as a dependency, use `Entity::fresh_from_index` which is identical to the previous `Entity::from_raw`, except that it now returns `Option` where the result is `None` if `u32::MAX` is passed.

Bevy made this change because it puts a niche in the `EntityRow` type which makes `Option<EntityRow>` half the size of `Option<u32>`.
This is used internally to open up performance improvements to the ECS.

Although you probably shouldn't be making entities manually, it is sometimes useful to do so for tests.
To migrate tests, use:

```diff
- let entity = Entity::from_raw(1);
+ let entity = Entity::fresh_from_index(1).unwrap();
```

If you are creating entities manually in production, don't do that!
Use `Entities::alloc` instead.
But if you must create one manually, either reuse a `EntityRow` you know to be valid by using `Entity::from_raw` and `Entity::row`, or handle the error case of `None` returning from `Entity::fresh_from_index(my_index)`.
