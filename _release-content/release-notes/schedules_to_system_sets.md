---
title: "Replace schedules with system sets"
pull_requests: [25858]
---

When it comes to Bevy scheduling there are thee concepts you are probably familiar with: schedules, system sets and systems. In 0.21 we change many of Bevy's internal schedules --- not all --- into system sets. Prime examples include `Update`, `Startup` and `FixedUpdate`.

If you've been here a while, you might remember ['base sets'](https://docs.rs/bevy/0.10.0/bevy/ecs/schedule/trait.SystemSet.html) which were introduced in 0.10 and [removed in 0.11](https://bevy.org/news/bevy-0-11/#schedule-first-ecs-apis) for a [host of reasons](https://github.com/bevyengine/bevy/pull/8079#base-set-confusion).

So what has changed in the last 3 years that made use decide to partially bring them back.

At the end of the day, both schedules and system sets are containers for systems, with the latter being far more flexible. Instead of having to use `MainScheduleOrder` in order to add your own schedule between `Update` and `PostUpdate`, you can now simply configure sets:

```rust
app.configure_sets(Main, (Update, MyUpdate, PostUpdate).chain());
```

where `Main` is the top-level schedule for the main world.

Additionally, this system set approach plays a lot better with the improved API first introduced in 0.11. The only implicit action taking place when putting a system in `Update` is that `Update` has a default schedule it exists in. In short:

```rust
// Because we define the default schedule...
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)] 
pub struct Update;

// ...we get that this...
app.add_systems(Update, my_system);

// ...is equivalent to this.
app.add_systems(Main, my_system.in_set(Update));
```

For the nitty gritty details, go and take a look at the migration guide. If, however, you're reading this and wonder when Bevy will finally stop tinkering with its scheduler... Well, I wouldn't bet on it.
