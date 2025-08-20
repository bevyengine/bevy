---
title: New `zstd` backend
pull_requests: [19793]
---

A more performant zstd backend has been added for texture decompression. To enable it, disable default-features and enable feature "zstd_c".
If you have default-features disabled and use functionality that requires zstd decompression ("tonemapping_luts" or "ktx2"), you must choose a zstd implementation with one of the following feature flags: "zstd_c" (faster) or "zstd_rust" (safer)
