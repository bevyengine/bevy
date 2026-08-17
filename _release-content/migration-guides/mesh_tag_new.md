---
title: `MeshTag(x)` is now `MeshTag::new(x)`
pull_requests: [24922]
---

Creating a new `MeshTag` is now done with e.g. `MeshTag::new(12345)` instead of e.g. `MeshTag(12345)`.

Likewise, accessing the value of a `MeshTag` is now done with `tag.value` instead of `tag.0`. (It still implements the `Deref` and `DerefMut` traits, so you can use the `*` operator instead if you wish.)

This was done because `MeshTag` now has an optional type ID (which you can supply with `MeshTag::with_type` if you wish). You can use this type ID to help identify instances in which your application accidentally overwrites one mesh tag with another. Note that the type ID is only stored in debug mode.
