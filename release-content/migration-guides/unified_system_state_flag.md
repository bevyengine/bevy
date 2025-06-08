---
title: Unified system state flag
pull_requests: [19506]
---

Now the system have a unified `SystemStateFlags` to represent its different states.

If your code previously looked like this:

```rust
impl System for MyCustomSystem {
    // ...

    fn is_send(&self) -> bool {
        false
    }

    fn is_exclusive(&self) -> bool {
        true
    }

    fn has_deferred(&self) -> bool {
        false
    }

    // ....
}
```

You should migrate it to:

```rust
impl System for MyCustomSystem{
  // ...

  fn flags(&self) -> SystemStateFlags {
    // non-send , exclusive , no deferred
    SystemStateFlags::NON_SEND | SystemStateFlags::EXCLUSIVE
  }

  // ...
}
```
