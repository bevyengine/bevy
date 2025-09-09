---
title: Remove `ArchetypeComponentId`
pull_requests: [19143]
---

Scheduling no longer uses `archetype_component_access` or `ArchetypeComponentId`.
To reduce memory usage and simplify the implementation, all uses of them have been removed.
Since we no longer need to update access before a system runs, `Query` now updates it state when the system runs instead of ahead of time.

`SystemParam::validate_param` now takes `&mut Self::State` instead of `&Self::State` so that queries can update their state during validation.

The trait methods `System::update_archetype_component_access` and `SystemParam::new_archetype` have been removed.
They are no longer necessary, so calls to them can be removed.
If you were implementing the traits manually, move any logic from those methods into `System::validate_param_unsafe`, `System::run_unsafe`, `SystemParam::validate_param`, or `SystemParam::get_param`, which can no longer rely on `update_archetype_component_access` being called first.

The following methods on `SystemState` have been deprecated:

* `update_archetypes` - Remove calls, as they no longer do anything
* `update_archetypes_unsafe_world_cell` - Remove calls, as they no longer do anything
* `get_manual` - Replace with `get`, as there is no longer a difference
* `get_manual_mut` - Replace with `get_mut`, as there is no longer a difference
* `get_unchecked_mut` - Replace with `get_unchecked`, as there is no longer a difference
