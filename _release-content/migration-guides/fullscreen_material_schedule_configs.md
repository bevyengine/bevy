---
title: "`FullScreenMaterial` API changes"
pull_requests: [23786]
---

`FullScreenMaterial::run_in`, `FullScreenMaterial::run_after` and `FullScreenMaterial::run_before` are replaced
by `FullScreenMaterial::schedule_configs` to configure the system order.

```rust
// 0.18
impl FullscreenMaterial for FullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        "shaders/fullscreen_effect.wgsl".into()
    }
    fn run_in() -> impl SystemSet {
        Core3dSystems::PostProcess
    }
    fn run_after() -> Option<Core3dSystems> {
        None
    }
    fn run_before() -> Option<Core3dSystems> {
        None
    }
}

// 0.19
impl FullscreenMaterial for FullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        "shaders/fullscreen_effect.wgsl".into()
    }
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .in_set(Core3dSystems::PostProcess)
            .before(tonemapping)
    }
}
```
