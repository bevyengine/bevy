---
title: `AnimationTarget` replaced by separate components.
pull_requests: [20774]
---

The `AnimationTarget` component has been split into two separate components:
`AnimationTargetId` and `AnimationPlayerTarget`. This makes it more flexible.

Before:

```rust
entity.insert(AnimationTarget { id: AnimationTargetId(id), player });
```

After:

```rust
entity.insert((AnimationTargetId(id), AnimationPlayerTarget(player)));
```
