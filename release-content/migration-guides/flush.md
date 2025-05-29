---
title: Entities APIs
pull_requests: [19350, 19433]
---

`Entities::flush` now also asks for metadata about the flush operation
that will be stored for the flushed entities. For the source location,
`MaybeLocation::caller()` can be used; the tick should be retrieved
from the world.
Additionally, flush now gives `&mut Option<EntityLocation>` instead of `&mut EntityLocation` access.
This replaces invalid locations with `None`.

`Entities::free` now returns `Option<Option<EntityLocation>>` rather than `Option<EntityLocation>`.
This is because invalid locations have been replaced with `None`.
So, if the outer option is none, the entity didn't exist, and if the inner option is none, the entity existed and was freed, but did not have a location.

`EntityMeta::location` has changed from `EntityLocation` to `Option<EntityLocation>` to replace invalid locations with `None`.
