---
title: "Some large error variants are boxed"
pull_requests: [24206, 24624]
---

Some large error variants are boxed to avoid `clippy::result_large_err`:

- `AssetLoadError::RequestedHandleTypeMismatch` now is a `Box<RequestedHandleTypeMismatchError>`
- `LoadDirectError::LoadError::error` now is a `Box<AssetLoadError>`
