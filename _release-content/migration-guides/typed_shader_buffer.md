---
title: "`ShaderBuffer` records are typed, and `GpuComponentArrayBuffer::Out` is a `GpuElement`"
pull_requests: []
---

`ShaderBuffer` has a new public field, `element_size`. Code that builds a `ShaderBuffer` with a struct literal needs `element_size: 0` or `..default()`.

`GpuComponentArrayBuffer::Out` is now bound by `GpuElement`: `Pod` plus `ShaderType`. Derive `ShaderType` on the type, and call `assert_gpu_layout!` for it to check that its Rust layout matches the shader's.
