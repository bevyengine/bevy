---
title: "`irradiance_volume_light` returns `IrradianceVolumeLight`"
pull_requests: []
---

`irradiance_volume_light` now returns `IrradianceVolumeLight` instead of a `vec3`.

```wgsl
// Before:
let light = irradiance_volume::irradiance_volume_light(world_position, N, &ranges);
color += light * diffuse_color;

// After:
let light = irradiance_volume::irradiance_volume_light(world_position, N, &ranges).irradiance;
color += light * diffuse_color;
```
