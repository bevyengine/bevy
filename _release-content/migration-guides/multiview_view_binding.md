---
title: View uniforms are now a packed array, and shaders index `view_array` instead of reading `view`
pull_requests: [24422]
---

Multiview camera support (rendering several subviews — e.g. two VR eyes — from one
camera) requires each camera to supply *N* view uniforms rather than one. The view
uniform buffer is now a packed array, and the WESL binding changed shape to match.

## Shaders: `view` is now `view_array[current_view_index]`

`bevy_pbr::render::mesh_view_bindings` no longer exposes a `view` binding. It now
exposes `view_array` plus a `current_view_index`, and shaders index the array:

```wgsl
// Before
let position = view.clip_from_world * vec4(world_position, 1.0);

// After
let position = view_array[current_view_index].clip_from_world * vec4(world_position, 1.0);
```

This affects any custom material or post-process shader that reads the view uniform,
including via an alias (`import ... mesh_view_bindings as view_bindings;` then
`view_bindings::view.viewport` ->
`view_bindings::view_array[view_bindings::current_view_index].viewport`). If you
imported the symbol by name, import `{view_array, current_view_index}` instead.

`current_view_index` is a `var<private>` that defaults to `0`, so single-view
rendering is unchanged.

Note the array is indexed at each use site rather than wrapped in a
`fn view() -> View` accessor: returning the ~784-byte `View` struct by value
miscompiles on Metal and produces visible corruption in shaders that read it
heavily.

## Shaders: custom entry points under multiview

If you render a multiview camera and write your own vertex or fragment entry point,
thread `@builtin(view_index)` into `current_view_index`, or every subview will read
subview 0's data:

```wgsl
@fragment
fn fragment(
    in: VertexOutput,
@if(MULTIVIEW)
    @builtin(view_index) view_index: i32,
) -> @location(0) vec4<f32> {
@if(MULTIVIEW)
    bevy_pbr::render::mesh_view_bindings::current_view_index = view_index;
    // ...
}
```

The default `mesh.wesl` vertex entry and `pbr.wesl` fragment entry already do this, so
a material that overrides only one of the two still gets correct per-subview behavior
from the default side.

## `ViewUniforms::uniforms` changed type

`DynamicUniformBuffer<ViewUniform>` -> `DynamicArrayUniformBuffer<ViewUniform>`, which
packs a runtime-sized array per dynamic-offset slot. If you wrote view uniforms
yourself, `get_writer(..)` is replaced by `clear()` / `push_array(..)` /
`finish_queuing()` / `write_buffer(..)`, and offsets are read back with
`get_array_offset(..)` after `finish_queuing()`.

Bind group layouts for the view binding should use
`uniform_buffer_sized(true, None)` instead of `uniform_buffer::<ViewUniform>(true)`, so
one layout accepts both the single-view fallback and the multiview array.

## `RenderPipelineDescriptor` gained a `multiview_mask` field

`RenderPipelineDescriptor` now has `multiview_mask: Option<NonZeroU32>`, matching the
render pass descriptor field of the same name. It defaults to `None`. Struct literals
that list every field exhaustively need to add it; anything using `..default()` is
unaffected.

## `prepass::get_bindings` gained parameters

```rust
// Before
get_bindings(prepass_textures)

// After
get_bindings(prepass_textures, multiview_array, deferred_multiview)
```

Pass `false` for both to keep the previous behavior. They select `D2Array` views of the
prepass textures for multiview cameras. Note that MSAA and multiview are mutually
exclusive for these bindings, because WGSL has no
`texture_depth_multisampled_2d_array`; pass `multiview_array` only when MSAA is off.
