---
title: Flushing
pull_requests: [19350]
---

`Entities::flush` now also asks for metadata about the flush operation
that will be stored for the flushed entities. For the source location,
`MaybeLocation::caller()` can be used; the tick should be retrieved
from the world.
