---
title: Schedule Build Pass API changes
pull_requests: [19450]
---

The schedule build pass API was improved to provide more context and flexabilities.

# Summary of Changes

- Breaking Change: The ScheduleBuildPass trait methods (`collapse_set` and `build`) have been updated with new parameters `(&mut World, &mut ScheduleGraph)` to provide more context.

- New Feature: A new method, `map_set_to_systems`, has been added to the ScheduleBuildPass trait, allowing passes to dynamically add systems to a set.


# Migration Steps
If you have custom implementations of `ScheduleBuildPass`, you will need to update the method signatures for the `collapse_set` and `build` methods by adding the `world` and `schedule` parameters.
