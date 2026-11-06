---
title: Many-to-Many Relationships
pull_requests: []
---

In order to support many-to-many relationships, the `Relationship` trait has been changed:

- Renamed `RelationshipSourceCollection` to `RelationshipCollection`, as its now also used for holding target entities.
- Renamed `OrderedRelationshipSourceCollection` to `OrderedRelationshipCollection`.
- Added a `Collection` associated type, similar to the one on `RelationshipTarget`. This means `Relationship` `Component`s now may point to more than one entity.
- `Relationship::get` has been renamed to `Relationship::collection` and returns a reference to the `Collection`.
- Added `Relationship::collection_mut_risky` for mutation of the held `Collection`, to facilitate updating a relationship with an additional target, or removing a target.
- Added `RelationshipCollection::from` for creating a collection from a single given entity.
- Removed `Relationship::set_risky` as its function is now fully covered by `Relationship::get_mut_risky`.

For existing users, you can migrate over to the new `SingleTargetRelationship = Relationship<Collection = Entity>` trait, which implements `SingleTargetRelationship::get` that returns an `Entity`.

For dynamic access: `RelationshipAccessorInitializer` and `RelationshipAccessor` have been changed to take a function pointer that returns an iterator over entities, instead of a field offset .
