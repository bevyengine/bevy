---
title: "Order independent transparency changes to support premultiplied alpha modes and per-material opt-out"
pull_requests: [22821, 24856]
---

1. OIT now can be opt-out per material via the new `enable_oit` method on `Material` and `MaterialExtension`. The associated `ShaderDef` is `MATERIAL_OIT_ENABLED`. The original `OIT_ENABLED` is used for `mesh_view_bindings.wgsl` and is enabled per camera/view. So custom oit-compatible material shaders should gate `oit_draw` behind `MATERIAL_OIT_ENABLED` instead of `OIT_ENABLED`.
2. The `oit_draw` function now expects alpha-premultiplied color to support `AlphaMode::Premultiplied` and `AlphaMode::Add` in addition to `AlphaMode::Blend` modes.

```wgsl
// BEFORE
#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // OIT_ENABLED

#ifdef OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED

// AFTER
#ifdef MATERIAL_OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // MATERIAL_OIT_ENABLED

#ifdef MATERIAL_OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
    // Both `Premultiplied` and `Add` colors are premultiplied in `premultiply_alpha()`
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // MATERIAL_OIT_ENABLED
```
