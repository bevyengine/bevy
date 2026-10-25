---
title: MetalFX ray reconstruction
authors: ["@mate-h"]
pull_requests: []
---

Solari on Apple Silicon can now denoise with MetalFX Temporal Denoised Scaler, the Metal counterpart to DLSS Ray Reconstruction. It needs macOS 26 or later and a GPU that supports the temporal denoised scaler.

Enable Bevy's `metal_fx` feature and add `MetalFx` to a camera when `MetalFxTemporalDenoisedScalerSupported` is present. The Solari example does this automatically and toggles it with `3`:

```sh
cargo run --example solari --features bevy_solari,metal_fx,https,free_camera
```
