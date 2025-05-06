---
title: Remove `ArchetypeComponentId`
pull_requests: [16885]
---

The schedule will now prevent systems from running in parallel if there *could* be an archetype that they conflict on, even if there aren't actually any.  For example, these systems will now conflict even if no entity has both `Player` and `Enemy` components:

```rust
fn player_system(query: Query<(&mut Transform, &Player)>) {}
fn enemy_system(query: Query<(&mut Transform, &Enemy)>) {}
```

To allow them to run in parallel, use `Without` filters, just as you would to allow both queries in a single system:

```rust
// Either one of these changes alone would be enough
fn player_system(query: Query<(&mut Transform, &Player), Without<Enemy>>) {}
fn enemy_system(query: Query<(&mut Transform, &Enemy), Without<Player>>) {}
```
