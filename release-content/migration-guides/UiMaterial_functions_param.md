---
title: UiMaterial shader functions param change
pull_requests: [20895]
---

`fn fragment_shader()` is now `fn fragment_shader(&self)` and `fn vertex_shader()` is now `fn vertex_shader(&self)` in `impl UiMaterial`, to allow accessing `&self`, which allows making the shaders dynamic