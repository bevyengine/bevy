---
title: "`CheckChangeTicks` parameter in `System::check_change_tick`"
pull_requests: [19274, 19600]
---

`System::check_change_tick` took a `Tick` parameter to update internal ticks. This is needed to keep queried components filtered by their change tick reliably not be matched if their last change or add and the system's last run was very long ago. This is also needed for similar methods involving the system's ticks for the same reason.

This parameter is now a `CheckChangeTicks` type that can be passed to the now-public `Tick::check_tick` in case you maintain these yourself in manual `System` implementations.

If you need a `CheckChangeTicks` value, for example because you call one of the above methods manually, you can observe it. Here is an example where it is used on a schedule stored in a resource, which will pass it on to the `System::check_change_tick` of its systems.

```rs
use bevy_ecs::prelude::*;
use bevy_ecs::component::CheckChangeTicks;

#[derive(Resource)]
struct CustomSchedule(Schedule);

let mut world = World::new();
world.add_observer(|check: On<CheckChangeTicks>, mut schedule: ResMut<CustomSchedule>| {
    schedule.0.check_change_ticks(*check);
});
```

The observers are triggered by `World::check_change_ticks` which every schedule calls before running. This method also returns an `Option<CheckChangeTicks>` which is `Some` in case it was time to check the ticks.
