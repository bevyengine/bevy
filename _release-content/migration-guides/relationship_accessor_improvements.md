---
title: RelationshipAccessor improvements
pull_requests: [23280]
---

`RelationshipAccessor` now has additional fields:

- `allow_self_referential` that stores value of `Relationship::ALLOW_SELF_REFERENTIAL`
- `relationship_target` for `RelationshipAccessor::Relationship` and `relationship` for `RelationshipAccessor::RelationshipTarget` which store `ComponentId` of the counterpart component.

`ComponentDescriptor::new_with_layout` now takes `Option<RelationshipAccessorInitializer>` instead of `Option<RelationshipAccessor>`, which requires providing a way to get `ComponentId` of the counterpart component.
