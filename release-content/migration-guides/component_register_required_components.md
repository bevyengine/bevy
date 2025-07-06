---
title: Refactor of Component::register_required_component
pull_requests: [19972]
---

The `register_required_component` method of the `Component` trait has been refactored to no longer be aware of the recursive registration, and in particular its `requiree`, `inheritance_depth` and `recursion_check_stack` parameters have been removed.
