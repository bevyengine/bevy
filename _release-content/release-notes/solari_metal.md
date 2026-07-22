---
title: Solari on Metal
authors: ["@mate-h"]
pull_requests: [25123]
---

Ray tracing on Metal has been available since wgpu 29, and with bindless storage buffers landing in wgpu 30, Solari now runs on Apple Silicon Macs.

Run the Solari example on a compatible Mac:

```
cargo run --example solari --features bevy_solari,https,free_camera
```

Denoising is not available on Metal yet, DLSS is NVIDIA-only. MetalFX Ray Reconstruction or Open Image Denoise 3.0 are promising paths for cross-platform denoising in the future.
