---
title: Renamed several `clear_children` to `detach_all_children`
pull_requests: [21470]
---

We renamed several related methods on both `EntityCommands` and `EntityWorldMut`:
- The method `clear_children` has been renamed to `detach_all_children`.
- the method `remove_children` to `detach_children`
- and the method `remove_child` to `detach_child`.

This should clarify that these methods do not despawn the child entities.
