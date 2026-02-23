---
title: Lifecycle observers include old and new archetypes
pull_requests: [22828]
---

Lifecycle observers now include information about the old and new archetypes during a change in the `EntityComponentsTrigger`.
As all of the fields for this `struct` are `pub`, adding new ones is a breaking change.  

If you were pattern matching the `components` field on `EntityComponentsTrigger`, you will need to add `..` to the pattern.

```rust
// 18.0
let EntityComponentsTrigger { components } = e.trigger();
// 19.0
let EntityComponentsTrigger { components, .. } = e.trigger();
```

If you were constructing an `EntityComponentsTrigger` manually, you will need to supply values for `old_archetype` and `new_archetype`.

```rust
// 18.0
world.trigger_with(
    event,
    EntityComponentsTrigger {
        components: &[component_a],
    },
);
// 19.0
world.trigger_with(
    event,
    EntityComponentsTrigger {
        components: &[component_a],
        old_archetype: None,
        new_archetype: None,
    },
);
```
