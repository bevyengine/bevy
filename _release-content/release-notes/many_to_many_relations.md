---
title: Many-to-Many Relationships
pull_requests: []
---

Bevy now supports many-to-many relations!

```rust
#[derive(Component)]
#[relationship(relationship_target = LikedBy)]
struct Likes(pub Vec<Entity>); // accepts any type that `LikedBy` would accept

#[derive(Component)]
#[relationship_target(relationship = Likes)]
struct LikedBy(Vec<Entity>);
```
