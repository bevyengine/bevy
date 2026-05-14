---
title: "`InputFocus` fields are no longer public"
pull_requests: [23723]
---

The `.0` field on `InputFocus` is no longer public.
Use the getter and setter methods instead.

In 0.18:

```rust
let focused_entity = input_focus.0;
input_focus.0 = Some(entity);
input_focus.0 = None;
```

In 0.19:

```rust
let focused_entity = input_focus.get();
input_focus.set(entity);
input_focus.clear();
```

Additionally, the core setup of `InputFocus` and related resources now occurs in `InputFocusPlugin`,
rather than `InputDispatchPlugin`.
This is part of `DefaultPlugins`, so most users won't need to make any changes.
