---
title: `MeshPipelineViewLayouts`, `MeshPipeline` and `RenderDebugOverlayPipeline` are now `RenderStartup` systems
pull_requests: [22443]
---

Systems using the `MeshPipelineViewLayouts`, `MeshPipeline` and `RenderDebugOverlayPipeline` resources in the `RenderStartup` schedule now need to be run after the `MeshPipelineSet` system set.
