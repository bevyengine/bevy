---
title: Entities APIs
pull_requests: [19350, 19433]
---

`Entities::flush` now also asks for metadata about the flush operation
that will be stored for the flushed entities. For the source location,
`MaybeLocation::caller()` can be used; the tick should be retrieved
from the world.

Additionally, flush now gives `&mut EntityIdLocation` instead of `&mut EntityLocation` access.
`EntityIdLocation` is an alias for `Option<EntityLocation>`.
This replaces invalid locations with `None`.
It is possible for an `Entity` id to be allocated/reserved but not yet have a location.
This is used in commands for example, and this reality is more transparent with an `Option`.
This extends to other interfaces: `Entities::free` now returns `Option<EntityIdLocation>` instead of `Option<EntityLocation>`.
`Entities::get` remains unchanged, but you can access an `Entity`'s `EntityIdLocation` through the new `Entities::get_id_location`.
