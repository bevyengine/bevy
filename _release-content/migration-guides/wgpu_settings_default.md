---
title: "`WgpuSettings` no longer contains `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` by default"
pull_requests: [25298]
---

`WgpuSettings::features` no longer contains `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` by default, which means adapter specific texture features won't be enabled if you use `WgpuSettingsPriority::WebGPU` or `WgpuSettingsPriority::WebGL2`.

You can manually enable the feature:

```rust
DefaultPlugins.set(RenderPlugin {
    render_creation: WgpuSettings {
        priority: WgpuSettingsPriority::WebGPU,
        features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        ..default()
    }
    .into(),
    ..Default::default()
})
```
