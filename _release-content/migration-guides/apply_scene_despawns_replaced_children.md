---
title: Applying a scene now despawns the related entities it replaces
pull_requests: []
---

Previously, applying a `Scene` to an existing entity (ex: via `apply_scene` or `queue_apply_scene`)
would replace and _orphan_ the entity's pre-existing related entities (ex: `Children`). The orphaned
entities lingered in the world, which produced "ghost" UI nodes when re-applying widget scenes
(ex: updating the rows of a `FeathersListView` with a scene patch).

Now, when an applied scene defines related entities for a relationship, the pre-existing related
entities are _despawned_ if the relationship uses "linked spawn" semantics
(see `RelationshipTarget::LINKED_SPAWN`). This is the case for `Children`, so re-applying a scene
that defines children replaces them cleanly. Relationships without "linked spawn" semantics still
orphan the replaced entities.

If you relied on the previous behavior, break the relationship before applying the scene (ex: by
calling `remove_children` or removing the relationship component) so the entities you want to keep
are no longer related when the scene is applied.
