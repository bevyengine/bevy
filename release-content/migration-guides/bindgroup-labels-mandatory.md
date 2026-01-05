---
title: "`BindGroupLayout` labels are no longer optional"
pull_requests: [21573]
---

In previous versions of Bevy, the `label` of a `BindGroupLayout` was optional. This practically only applies when implementing `AsBindGroup` manually without the `AsBindGroup` derive.

If you were previously omitting the `label` implementation from a `impl AsBindGroup`, you now must implement it:

```rust
impl AsBindGroup for CoolMaterial {
    // ...

    fn label() -> &'static str {
        // It is customary to make the label the name of the type.
        "CoolMaterial"
    }
}
```
