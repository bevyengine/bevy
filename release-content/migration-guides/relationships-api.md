---
title: Many-to-Many Relationships
pull_requests: [20377]
---

In order to support many-to-many relationships, the `Relationship` trait has been changed:

- Renamed `RelationshipSourceCollection` to `RelationshipCollection`, as its now also used for holding target entities.
- Renamed `OrderedRelationshipSourceCollection` to `OrderedRelationshipCollection`.
- Added a `Collection` associated type, similar to the one on `RelationshipTarget`. This means `Relationship` `Component`s now may point to more than one entity.
- `Relationship::get` now returns a reference to the `Collection`, which means one-to-many or one-to-one relationships now receive an `&Entity`.
- Added `Relationship::get_mut_risky` for mutation of the held `Collection`, to facilitate updating a relationship with an additional target, or removing a target.
- Added `RelationshipCollection::from` for creating a collection from a single given entity.

If needing to support only one-to-one or one-to-many relationships in trait bounds, specify `T: Relationship<Collection = Entity>`.
