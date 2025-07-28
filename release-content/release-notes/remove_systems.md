---
title: Remove Systems from Schedules
authors: ["@hymm"]
pull_requests: [20298]
---

A long requested feature has come to Bevy! You can now remove systems from a schedule. The previous recommened way of preventing a system from running was to use RunConditions. This is still the recommended way for most situations, because actually removing the system causes the schedule to be rebuilt. This can potentially be slow as a bunch of graph logic needs to be rechecked. But for situations where this is not a problem, you can now call `remove_systems_in_set`.

```rust
app.add_systems((system_a, (system_b, system_c).in_set(MySet)));

// remove a system
app.remove_systems_in_set(system_a);

// remove systems in a set
app.remove_systems_in_set(MySet)
```


