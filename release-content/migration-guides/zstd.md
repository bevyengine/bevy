---
title: New `zstd` backend
pull_requests: [19793]
---

A more performant zstd backend has been added for texture decompression. To enable it, disable default-features and enable feature "zstd_c".
If you have default-features disabled and use functionality that requires zstd decompression ("tonemapping_luts" or "ktx2"), you must choose a zstd implementation with one of the following feature flags: "zstd_c" (faster) or "zstd_rust" (safer)

## Migration Guide

If you previously used the `zstd` feature explicitly:
```toml
# 0.16
[dependencies]
bevy = { version = "0.16", features = ["zstd"] }

# 0.17 - Use the safe Rust implementation:
[dependencies]
bevy = { version = "0.17", features = ["zstd_rust"] }

# 0.17 - Or use the faster C implementation:
[dependencies]
bevy = { version = "0.17", features = ["zstd_c"] }
```

If you have default-features disabled and use functionality that requires zstd decompression ("tonemapping_luts" or "ktx2"), you must choose a zstd implementation with one of the following feature flags: "zstd_c" (faster) or "zstd_rust" (safer).