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

`CheckChangeTicks` observers now run twice per check, and should call `check_tick` both times. `World::check_change_ticks` now returns `bool` instead of `Option<CheckChangeTicks>`, and only clamps ticks while a schedule or exclusive system is running.

`Tick`s you store yourself, like the last run of a `SystemState` you keep around, should be passed to `check_tick` by a `CheckChangeTicks` observer. Without one, they now compare wrongly once they're about 1.7 billion ticks old, instead of about 3.3 billion. If you clamped ticks yourself using `present_tick`, call `check_tick` instead.

`RegisteredSystem<I, O>` now requires `I: SystemInput`.
