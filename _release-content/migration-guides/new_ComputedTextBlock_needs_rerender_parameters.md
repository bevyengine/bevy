---
title: "New ComputedTextBlock::needs_rerender parameters"
pull_requests: [22614]
---

`ComputedTextBlock::needs_rerender` takes two new bool parameters: `is_viewport_size_changed` and `is_rem_size_changed`.

`is_viewport_size_changed` should be true if the local viewport size has changed this frame, and `is_rem_size_changed` should be true if the rem size (probably corresponding to the value of the `RemSize` resource) has changed this frame.
