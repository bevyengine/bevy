---
title: "Merged sorted phase items have an empty `batch_range`"
pull_requests: []
---

`SortedRenderPhase::render_range` draws every item with a non-empty `batch_range` and advances one item at a time. It no longer skips `batch_range.len()` items after a batched draw.

The built-in batchers now give items merged into an earlier item's batch an empty `batch_range`. Custom code that merges sorted phase items must do the same instead of relying on the first item's range length to skip the merged items.
