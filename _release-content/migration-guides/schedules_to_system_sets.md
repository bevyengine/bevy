---
title: "Replace schedules with system sets"
pull_requests: [25858]
---

In 0.21, the following `Schedule`s were turned into system sets: `PreStartup`, `Startup`, `PostStartup`, `First`, `PreUpdate`, `Update`, `PostUpdate`, `Last`, `RunFixedMainLoop`, `FixedFirst`, `FixedPreUpdate`, `FixedUpdate`, `FixedPostUpdate`, and `FixedLast`.

If you've been defining your own schedules, we recomened switching over to system sets as well:

```rust
// 0.20
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct MyUpdate;

// 0.21
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct MyUpdate;
```

Here, `#[default_schedule(...)]` specifies in what schedule this set belongs. Because of this, the following are equivalent:

```rust
app.add_systems(MyUpdate, (system_a, system_b, etc));
// is the same as
app.add_systems(Main, (system_a, system_b).in_set(MyUpdate));
```

This means that schedules still exist and the most common ones you deal with are `Main`, `FixedMain`, and `StartupMain`.

Additionally, `FixedMainScheduleOrder` and `MainScheduleOrder` have been removed. So if you need to put your system set before or after another one, you can use the usual ordering machinery:

```rust
// 0.20
app.world_mut()
    .resource_mut::<MainScheduleOrder>()
    .insert_after(Update, MyUpdate);

// 0.21
app.configure_sets(Main, (Update, MyUpdate, PostUpdate).chain()));
```
