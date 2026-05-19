---
title: Observer Ordering
authors: ["@caniko"]
pull_requests: [24328]
---

Observers can now be ordered against each other with the same `SystemSet`-style API used for systems. Iteration order is also now deterministic for callers who never opt into ordering — previously, hash-bucket order was undefined.

```rust
#[derive(ObserverSet, Hash, PartialEq, Eq, Clone, Copy, Debug)]
struct WinCheck;

app.add_observer(score.in_set(WinCheck));
app.add_observer(announce.after(WinCheck));
app.configure_observer_sets((WinCheck, Announce).chain());
```

Ordering edges work across dispatch buckets, so a global observer can be ordered against a per-entity or per-component observer for the same event. Internally each event type owns one topo-sorted observer graph, and dispatch sites walk that single order via a k-way merge over the active streams — no per-dispatch allocation.

Existing observer features keep working alongside the new ordering:

- `.run_if(condition)` composes with `.in_set(...)` / `.before(...)` / `.after(...)`, so a conditionally skipped observer no longer breaks the order of its neighbors.
- The archetype-flag fast skip for lifecycle component observers is preserved.

The builder surface (`.in_set`, `.before`, `.after`, `.chain`, `.with_name`) is available on every observer entry point: `World::add_observer` / `add_observers`, `Commands::add_observer`, `EntityWorldMut::observe`, `EntityCommands::observe`, and the free `entity_command::observe`. Tuples of observers can be modified together — `(a, b, c).chain().in_set(MySet)` applies the chain edges and set membership to all three.
