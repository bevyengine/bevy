---
title: "DLSS Super Resolution no longer uses DLSS auto exposure"
pull_requests: [26211]
---

DLSS Super Resolution now takes its exposure from the renderer instead of measuring the image
itself. A custom pass that changes a view's tonemapping exposure on the GPU should write a
`ViewExposureTexture` for that view.
