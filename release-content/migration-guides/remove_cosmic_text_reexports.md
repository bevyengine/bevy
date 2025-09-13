---
title: "Removed `cosmic_text` re-exports"
pull_requests: [19516]
---

Previously, `bevy_text` re-exported the entirety of `cosmic_text` while renaming a few of the most confusing re-exports,
using the following code.

```rust
pub use cosmic_text::{
    self, FamilyOwned as FontFamily, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight,
};
```

These re-exports commonly conflicted with other types (like `Query`!), leading to messy autocomplete errors.
Ultimately, these are largely an implementation detail, and were not widely used.

We've completely removed these re-exports (including the renamed types): if you need to use these types, please rely on them directly from `cosmic_text`, being sure that the version number you are using matches the version used by your version of `bevy_text`.
