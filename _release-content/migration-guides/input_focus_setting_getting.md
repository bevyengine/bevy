---
title: "`InputFocus` fields are no longer public"
pull_requests: [23723]
---

The `.0` field on `InputFocus` is no longer public.
Use the getter and setters methods instead.

Before:

```rust
let focused_entity = input_focus.0;
input_focus.0 = Some(entity);
input_focus.0 = None;
```

After:

```rust
let focused_entity = input_focus.get();
input_focus.set(entity);
input_focus.clear();
```

Additionally, the core setup of `InputFocus` and related resources now occurs in `InputFocusPlugin`,
rather than `InputDispatchPlugin`.
This is part of `DefaultPlugins`, so most users will not need to make any changes.
