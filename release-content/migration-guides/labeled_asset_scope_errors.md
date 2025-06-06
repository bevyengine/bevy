---
title: `labeled_asset_scope` can now return errors.
pull_requests: [19449]
---

`labeled_asset_scope` now returns a user-specified error type based on their closure. Previously,
users would need to fall back to `begin_labeled_asset` and `add_loaded_labeled_asset` to handle
errors, which is more error-prone. Consider migrating to use `labeled_asset_scope` if this was you!

However, `labeled_asset_scope` closures that don't return errors now needs to A) return Ok, and B)
specify an error type.

If your code previously looked like this:

```rust
labeled_asset_scope(label, |mut load_context| {
  let my_asset = ...;

  my_asset
});
```

You can migrate it to:

```rust
labeled_asset_scope::<_, ()>(label, |mut load_context| {
  let my_asset = ...;

  Ok(my_asset)
}).unwrap();
```
