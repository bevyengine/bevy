---
title: Remove Systems from Schedules
authors: ["@hymm"]
pull_requests: [20298]
---

A long requested feature has come to Bevy! You can now remove systems from a schedule.
The previous recommended way of preventing a scheduled system from running was to use `RunCondition`'s.
You will still use this for most situations as removing a system will cause the schedule to be rebuilt.
This process can be slow since the schedule checking logic is complex. But in situations where this is
not a problem, you can now call `remove_systems_in_set`. The advantage of this is that this will remove the
cost of the run condition being checked.

```rust
app.add_systems((system_a, (system_b, system_c).in_set(MySet)));

// remove a system
schedule.remove_systems_in_set(my_system, ScheduleCleanupPolicy::RemoveSystemsOnly);

// remove systems in a set
app.remove_systems_in_set(MySet, ScheduleCleanupPolicy::RemoveSetAndSystems);
```
