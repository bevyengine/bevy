---
title: Mesh view bind group layout is changed
pull_requests: [23982]
---

`MeshPipelineViewLayouts` no longer stores all possible view layouts. Now it only stores necessary parameters for creating bind group layouts on demand. And `MeshPipelineViewLayouts::get_view_layout` returns `MeshPipelineViewLayout`
by value instead of by reference.

`generate_view_layouts` is removed and `layout_entries` is private now. Please use `MeshPipelineViewLayouts::get_view_layout`.

Mesh view bind group layout has more variants now and some dynamic uniforms such as distance fog, ssr, contact shadows,
environment map are not guaranteed to exist. Please use `MeshViewBindGroup::main_offsets` to get the dynamic offsets.

Before:

```rust
    let mut offsets: SmallVec<[u32; 8]> = smallvec![
        view_uniform_offset.offset,
        view_lights_offset.offset,
        view_fog_offset.offset,
        **view_light_probes_offset,
        **view_ssr_offset,
        **view_contact_shadows_offset,
        **view_environment_map_offset,
    ];
    if let Some(oit_settings_offset) = maybe_oit_settings_offset {
        offsets.push(oit_settings_offset.offset);
    }
    pass.set_bind_group(I, &mesh_view_bind_group.main, &offsets);
```

After:

```rust
    pass.set_bind_group(
        I,
        &mesh_view_bind_group.main,
        &mesh_view_bind_group.main_offsets,
    );
```
