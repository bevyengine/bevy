---
title: Render Assets diagnostics
authors: ["@hukasu"]
pull_requests: [19311]
---

## Goals

Create diagnostics plugins `MeshAllocatorDiagnosticPlugin`, `MaterialDiagnosticPlugin` and `RenderAssetDiagnosticPlugin`
that collect measurements related to `MeshAllocator`s, `MaterialBindGroupAllocator`, and `RenderAssets` respectively.

`MeshAllocatorDiagnosticPlugin` and `MaterialDiagnosticPlugin` measure the number of slabs, the total size of memory
allocated by the slabs, and the number of objects allocated in the slabs. Only bindless materials use slabs for their
allocations, non-bindless materials return 0 for all of them.

`RenderAssetDiagnosticsPlugin` measure the number of assets in `RenderAssets<T>`.
