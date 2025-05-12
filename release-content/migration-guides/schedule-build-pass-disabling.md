---
title: Disabling `ScheduleBuildPass` and new trait method
pull_requests: [19195]
---

The method `Schedule::remove_build_pass` was renamed to `Schedule::disable_build_pass` to express the pass is still stored, just not applied.

The trait `ScheduleBuildPass` has a new method `combine` to combine two values of the same pass to prevent losing information that is in either of the two instances.

```rust
// 0.16
impl ScheduleBuildPass for MyBuildPass {
    //...
}

my_schedule.remove_build_pass::<MyBuildPass>();

// 0.17
impl ScheduleBuildPass for MyBuildPass {
    fn combine(&mut self, other: Self) {
        // add data from `other` to `self` if it is important for building the schedule
    }

    //...
}

my_schedule.disable_build_pass::<MyBuildPass>();
```
