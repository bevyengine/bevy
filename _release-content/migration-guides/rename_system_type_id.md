---
title: Rename `System::type_id` to `System::system_type`
pull_requests: [23326]
---

`System::type_id` has been renamed to `System::system_type` to avoid shadowing `Any::type_id`.

The old `System::type_id` method is now deprecated and will be removed in a future release. Replace all calls to `System::type_id` with `System::system_type`:

```rust
// 0.18
let id = my_system.type_id();

// 0.19
let id = my_system.system_type();
```

If you have a custom `System` implementation that overrides `type_id`, rename it to `system_type`.
