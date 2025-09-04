---
title: "OpenGL ES `wgpu` backend is no longer supported by default"
pull_requests: [ 20793 ]
---

The `gles` backend for `wgpu` is no longer included as a default feature of `bevy_render`. OpenGL support is still
available, but must be explicitly enabled by adding the `bevy_render/gles` feature to your app. This change reflects the
fact that OpenGL support is not tested and that some features may not work as expected or at all. We welcome
contributions to improve OpenGL support in the future.
