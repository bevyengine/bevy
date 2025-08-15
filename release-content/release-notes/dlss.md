---
title: Deep Learning Super Sampling (DLSS)
authors: ["@JMS55", "@cart"]
pull_requests: [19864, 19817, 20565]
---

For users with NVIDIA RTX GPUs, Bevy now offers yet another form of anti-aliasing: DLSS.

Try it out by running Bevy's anti_aliasing example: `cargo run --example anti_aliasing --features dlss --release` (after performing setup from <https://github.com/bevyengine/dlss_wgpu>).

Additionally, we've open sourced <https://github.com/bevyengine/dlss_wgpu> as a standalone crate to help other wgpu-based renderers integrate DLSS.

Compared to Bevy's built-in TAA, DLSS:

* Is much higher quality
* Supports upscaling in addition to anti-aliasing, leading to much cheaper render times, particularly when used with GPU-heavy features like Bevy Solari
* Requires a NVIDIA RTX GPU
* Requires running via the Vulkan backend on Windows/Linux (no macOS, web, or mobile support)

To use DLSS in your app:

* See <https://github.com/bevyengine/dlss_wgpu> for licensing requirements and setup instructions
* Enable Bevy's `dlss` feature
* Insert the `DlssProjectId` resource before `DefaultPlugins` when setting up your app
* Check for the presence of `Option<Res<DlssSuperResolutionSupported>>` at runtime to see if DLSS is supported on the current machine
* Add the `Dlss` component to your camera entity, optionally setting a specific `DlssPerfQualityMode` (defaults to `Auto`)
* Optionally add sharpening via `ContrastAdaptiveSharpening`
* Custom rendering code, including third party crates, should account for the optional `MainPassResolutionOverride` to work with DLSS (see the `custom_render_phase` example)

Note that DLSS integration is expected to have some bugs in this release related to certain rendering effects not respecting upscaling settings, and possible issues with transparencies or camera exposure. Please report any bugs encountered.

Other temporal upscalers like AMD's FidelityFX™ Super Resolution (FSR), Intel's Xe Super Sampling XeSS (XeSS), and Apple's MTLFXTemporalScaler are not integrated in this release. However they all use similar APIs, and would not be a challenge to integrate in future releases.

Support for other swapchain-related features like frame interpolation/extrapolation, latency reduction, or dynamic resolution scaling are not currently planned, but support for DLSS Ray Reconstruction for use in Bevy Solari _is_ planned for a future release.

Special thanks to @cwfitzgerald for helping with the [`wgpu`](https://github.com/gfx-rs/wgpu) backend interop APIs.
