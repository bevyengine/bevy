---
title: "`CheckChangeTicks` is passed by reference"
pull_requests: [26501]
---

Old change ticks are now compacted instead of clamped, so systems that haven't run in a long time no longer miss changes (and no longer log a warning).

`CheckChangeTicks` is no longer `Copy`. `Tick::check_tick`, `System::check_change_tick` and `Schedule::check_change_ticks` now take `&CheckChangeTicks`.

```rust
// 0.20
world.add_observer(|check: On<CheckChangeTicks>, mut schedule: ResMut<CustomSchedule>| {
    schedule.0.check_change_ticks(*check);
});

// 0.21
world.add_observer(|check: On<CheckChangeTicks>, mut schedule: ResMut<CustomSchedule>| {
    schedule.0.check_change_ticks(&check);
});
```

`CheckChangeTicks` observers now run twice per check, and should call `check_tick` both times. `World::check_change_ticks` now returns `bool` instead of `Option<CheckChangeTicks>`, and does nothing while a schedule is running.

`RegisteredSystem<I, O>` now requires `I: SystemInput`.
