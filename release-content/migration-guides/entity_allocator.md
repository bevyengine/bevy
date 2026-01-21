---
title: "`World::entities_allocator` is now `World::entity_allocator`"
pull_requests: [22638]
---

`World::entities_allocator()` has been renamed to `World::entity_allocator()` to match the type returned (`EntityAllocator`). Likewise, `World::entities_allocator_mut()` has been renamed to `World::entity_allocator_mut()`.
