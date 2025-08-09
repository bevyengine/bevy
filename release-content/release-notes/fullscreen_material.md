---
title: Fullscreen Material
authors: ["@IceSentry"]
pull_requests: [20414]
---

Users often want to run a fullscreen shader but currently the only to do this is to copy the custom_post_processing example which is very verbose and contains a lot of low level details. We introduced a new `FullscreenMaterial` trait and `FullscreenMaterialPlugin` that let you easily run a fullscreen shader and specify in which order it will run relative to other render passes in the engine.
