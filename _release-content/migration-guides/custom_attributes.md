---
title: "`CustomAttributes::with_attribute` has been replaced by a builder"
pull_requests: [24171]
---

Previously, `CustomAttributes` were created like this:

```rust
let custom_attributes = CustomAttributes::default()
    .with_attribute("my attribute");
    .with_attribute(123);
```

Now, `CustomAttributes` are created with `CustomAttributesBuilder`:

```rust
let custom_attributes = CustomAttributesBuilder::new()
    .attribute("my attribute")
    .attribute(123)
    .build();
```

This change was a side effect of memory optimizations internal to
`CustomAttributes`.
