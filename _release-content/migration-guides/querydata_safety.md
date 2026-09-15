---
title: Safety requirements for `QueryData::ReadOnly` have been clarified
pull_requests: [25652]
---

The safety requirements for `QueryData::ReadOnly` have been reworded,
Any existing sound implementation should still be sound,
but manual implementations of `QueryData` will want to update their `SAFETY` comments to match the new wording.

The type system constraint that `ReadOnly::State == Self::State` has been relaxed
in favor of a safety constraint that `Self::State` can be *transmuted* to `ReadOnly::State`.

In addition, the safety requirements are now explicit that the state must be a *valid* value of `ReadOnly::State`.
For example, `&T` has `type State = ComponentId;`,
and has always required that the value be the `ComponentId` of `T`,
but this was not noted explicitly in the safety requirements.

Read-only `QueryData` that has `type ReadOnly = Self;` can use
```rust
// SAFETY: `Self` is the same as `Self::ReadOnly`
```
