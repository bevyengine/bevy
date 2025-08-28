---
title: "`AnimationTarget` replaced by separate components"
pull_requests: [20774]
---

The `AnimationTarget` component has been split into two separate components.
`AnimationTarget::id` is now an `AnimationTargetId` component, and
`AnimationTarget::player` is now an `AnimatedBy` component.

This change was made to add flexibility. It's now possible to calculate the
`AnimationTargetId` first, but defer the choice of player until later.

Before:

```rust
entity.insert(AnimationTarget { id: AnimationTargetId(id), player: player_entity });
```

After:

```rust
entity.insert((AnimationTargetId(id), AnimatedBy(player_entity)));
```
