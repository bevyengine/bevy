---
title: Required components refactor
pull_requests: [20110]
---

The required components feature has been reworked to be more consistent around the priority of the required components and fix some soundness issues. In particular:

- the priority of required components will now always follow a priority given by the depth-first/preorder traversal of the dependency tree. This was mostly the case before with a couple of exceptions that we are now fixing:
  - when deriving the `Component` trait, sometimes required components at depth 1 had priority over components at depth 2 even if they came after in the depth-first ordering;
  - registering runtime required components followed a breadth-first ordering and used the wrong inheritance depth for derived required components.
- uses of the inheritance depth were removed from the `RequiredComponent` struct and from the methods for registering runtime required components, as it's not unused for the depth-first ordering;
- `Component::register_required_components`, `RequiredComponents::register` and `RequiredComponents::register_by_id` are now `unsafe`;
- `RequiredComponentConstructor`'s only field is now private for safety reasons.
