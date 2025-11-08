---
title: API for working with `Relationships` and `RelationshipTargets` in type-erased contexts
pull_requests: [21601]
---

`ComponentDescriptor` now stores additional data for working with relationships in dynamic contexts.
This resulted in changes to `ComponentDescriptor::new_with_layout`:

- Now requires additional parameter `relationship_accessor`, which should be set to `None` for all existing code creating `ComponentDescriptors`.
