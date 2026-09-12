---
title: "`ViewTarget::compositing_space` is replaced by `ResolvedCompositingSpace`"
pull_requests: [25481]
---

Cameras that composite over each other on one render target now share one
compositing space, the single space any of them requests through
`CompositingSpace`. A camera that clears starts a new stack and resolves on its
own. A stack whose cameras request conflicting spaces falls back to linear
compositing and logs a warning.

`ViewTarget::compositing_space` and `ExtractedCamera::compositing_space` have
been removed. Render-world code should query `Option<&ResolvedCompositingSpace>`
on the view entity instead.
