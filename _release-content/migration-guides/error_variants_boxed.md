---
title: "Some variants in `GltfError` and `LoadDirectError` are boxed"
pull_requests: [24206]
---

The values in `GltfError::ReadAssetBytesError`, `GltfError::AssetLoadError`
and `LoadDirectError::LoadError::error` are boxed now.
