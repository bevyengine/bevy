---
title: Self-Referential Relationships
authors: ["@mrchantey"]
pull_requests: [22269]
---

By default, Bevy rejects relationship components that point to the entity they live on. If you insert one, Bevy will log a warning and remove it.
This default exists for good reason: structural relationships like `ChildOf` form hierarchies that Bevy traverses recursively — a self-referential `ChildOf` would produce an infinite loop.

But many relationships are purely semantic. `Likes(self)`, `EmployedBy(self)`, `Healing(self)` — these don't imply any traversal, and self-reference is perfectly valid. You can now opt in with `allow_self_referential`:

```rust
#[derive(Component)]
#[relationship(relationship_target = PeopleILike, allow_self_referential)]
pub struct LikedBy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = LikedBy)]
pub struct PeopleILike(Vec<Entity>);
```

With the attribute set, inserting a self-referential relationship is accepted without warning.
