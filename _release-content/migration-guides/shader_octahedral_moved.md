---
title: "Shader `bevy_pbr::utils::{octahedral_encode, octahedral_decode, octahedral_decode_signed}` are moved"
pull_requests: [21926]
---

Shader functions `bevy_pbr::utils::{octahedral_encode, octahedral_decode, octahedral_decode_signed}` are moved to `bevy_render::utils::{octahedral_encode, octahedral_decode, octahedral_decode_signed}`

```wgsl
// BEFORE
#import bevy_pbr::utils::{octahedral_encode, octahedral_decode, octahedral_decode_signed}

// AFTER
#import bevy_render::utils::{octahedral_encode, octahedral_decode, octahedral_decode_signed}
```
