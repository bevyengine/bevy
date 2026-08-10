---
title: WESL Shaders
pull_requests: []
---

Bevy's shaders are now written in [WESL](https://wesl-lang.dev) and the
naga_oil preprocessor is gone. Custom shaders in the naga_oil dialect need to
be translated to WESL and renamed from `.wgsl` to `.wesl`. Plain WGSL files
with no preprocessor directives keep working.

```wgsl
// BEFORE
#import bevy_pbr::forward_io::VertexOutput
#import "shaders/util.wgsl"::hsv_to_rgb

#ifdef VERTEX_COLORS
var<private> tint: vec4<f32>;
#endif

@group(2) @binding(#{MATERIAL_BINDING}) var<uniform> color: vec4<f32>;

// AFTER
import bevy_pbr::render::forward_io::VertexOutput;
import super::util::hsv_to_rgb;

@if(VERTEX_COLORS)
var<private> tint: vec4<f32>;

@group(2) @binding(constants::MATERIAL_BINDING) var<uniform> color: vec4<f32>;
```

- Imports end with a semicolon and come first in the file, before any
  declaration or `enable` directive.
- Module names now match the shader's path in its crate:
  `bevy_pbr::mesh_view_bindings` is `bevy_pbr::render::mesh_view_bindings`,
  `bevy_pbr::prepass_utils` is `bevy_pbr::prepass::utils`, and so on.
- `@if`/`@elif`/`@else` attach to whole declarations, struct members, function
  parameters, imports and statements. Boolean shader defs become conditional
  compilation flags, and `Int`/`UInt` defs are readable as `constants::NAME`
  and enable a flag of the same name.
- `#define_import_path` is gone. Shaders loaded from `embedded://` are
  importable at their crate and file path (`embedded://bevy_foo/bar.wesl` is
  `bevy_foo::bar`), anything else at its asset path.

The `shader_format_wesl` cargo feature is gone, WESL support is always
enabled. GLSL support has also been removed. SPIR-V passthrough is unchanged.
