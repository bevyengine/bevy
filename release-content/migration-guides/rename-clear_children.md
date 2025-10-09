---
title: Renamed several `clear_children` to `detach_all_children`
pull_requests: [21470]
---

In summary, we renamed `clear_*` and `remove_*` methods to `detach_*`.
This should clarify that these methods do not despawn the child entities.

We renamed several related methods on both `EntityCommands` and `EntityWorldMut`:

- The method `EntityCommands::clear_children` has been renamed to `EntityCommands::detach_all_children`.
- The method `EntityWorldMut::clear_children` has been renamed to `EntityWorldMut::detach_all_children`.
- The method `EntityCommands::remove_children` has been renamed to `EntityCommands::detach_children`.
- The method `EntityWorldMut::remove_children` has been renamed to `EntityWorldMut::detach_children`.
- The method `EntityCommands::remove_child` has been renamed to `EntityCommands::detach_child`.
- The method `EntityWorldMut::remove_child` has been renamed to `EntityWorldMut::detach_child`.
