---
title: Gpu Transfer Priorities
authors: ["@robtfm"]
pull_requests: [22557]
---

Bevy has supported a throttle on gpu transfers to avoid frame hitches (particularly in wasm) since 0.14. But the feature was simplistic, supporting only a fixed number of bytes per frame.

This leads to problems like some assets never transferring due to the whole budget being taken by other assets being updated every frame, more important assets being stuck behind less important ones, and even essential resources like font atlases not being available when needed, resulting in flickering text when large assets are loaded.

The `RenderAssetBytesPerFrame` resource now supports a `MaxBytesWithPriority` mode. Individual assets can be assigned a priority of `Immediate` or a `Priority(i16)` priority level. This means essential resources can be guaranteed to update the same frame, and you can prioritise other assets as you choose.

This also means we can relax a previous constraint: when images or materials are modified, previously we would remove the old version straight away, to avoid shaders that expected a certain shape of data from crashing when presented with stale data. Now we have a tool to avoid this problem - these assets can use `Immediate` transfer mode - so we can keep the previous asset until the new one is available. This means modifying a material will no longer cause the previous rendered
This also means we can relax a previous constraint: when images or materials are modified, previously we would remove the old version straight away, to avoid shaders that expected a certain shape of data from crashing when presented with stale data. Now we have a tool to avoid this problem - these assets can use `Immediate` transfer mode - so we can keep the previous asset until the new one is available. This means modifying a material will no longer cause the previous material to disappear, instead it will continue to render, and only be replaced when the new version is available and transferred.

For a demo of the behaviour, see the new `gpu_transfer_limits` example.