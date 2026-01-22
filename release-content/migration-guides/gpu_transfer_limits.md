---
title: Gpu Transfer Priorities
pull_requests: [22557]
---

`RenderAssetTransferPriority` has been added to constructors for `Mesh` and `Image`. If you're using `RenderAssetBytesPerFrame::MaxPriorityWithBytes` you can set an appropriate priority here, otherwise the setting has no effect, so just pass `Default::default()` on each constructor.
