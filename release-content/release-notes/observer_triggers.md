---
title: Rename `Trigger` to `On`
authors: ["@Jondolf"]
pull_requests: [19596]
---

In past releases, the observer API looked like this:

```rust
app.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Added player {}", trigger.target());
});
```

In this example, the `Trigger` type contains information about the `OnAdd` event that was triggered
for a `Player`.

**Bevy 0.17** renames the `Trigger` type to `On`, and removes the `On` prefix from lifecycle events
such as `OnAdd` and `OnRemove`:

```rust
app.add_observer(|trigger: On<Add, Player>| {
    info!("Added player {}", trigger.target());
});
```

This significantly improves readability and ergonomics, and is especially valuable in UI contexts
where observers are very high-traffic APIs.

One concern that may come to mind is that `Add` can sometimes conflict with the `core::ops::Add` trait.
However, in practice these scenarios should be rare, and when you do get conflicts, it should be straightforward
to disambiguate by using `ops::Add`, for example.
