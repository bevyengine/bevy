---
title: Self-Referential Relationships
authors: ["@mrchantey"]
pull_requests: []
---

Relationships can now optionally point to their own entity by setting the `allow_self` attribute on the `#[relationship]` macro.

By default pointing a relationship to its own entity will log a warning and remove the component. However, self-referential relationships are semantically valid in many cases: `Likes(self)`, `EmployedBy(self)`, `TalkingTo(self)`, `Healing(self)`, and many more.

## Usage

To allow a relationship to point to its own entity, add the `allow_self` attribute:

```rust
#[derive(Component)]
#[relationship(relationship_target = PeopleILike, allow_self)]
pub struct LikedBy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = LikedBy)]
pub struct PeopleILike(Vec<Entity>);
```

Now entities can have relationships that point to themselves:

```rust
let entity = world.spawn_empty().id();
world.entity_mut(entity).insert(LikedBy(entity));

// The relationship is preserved
assert!(world.entity(entity).contains::<LikedBy>());
assert!(world.entity(entity).contains::<PeopleILike>());
```
