---
title: Resources `MeshPipelineViewLayouts`, `MeshPipeline` and `RenderDebugOverlayPipeline` are now created in `RenderStartup` systems
pull_requests: [22443]
---

Systems using the `MeshPipelineViewLayouts`, `MeshPipeline` and `RenderDebugOverlayPipeline` resources in the `RenderStartup` schedule now need to be run after the `MeshPipelineSet` system set.
