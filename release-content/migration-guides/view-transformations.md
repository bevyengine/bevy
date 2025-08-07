---
title: view_transformations.wgsl deprecated in favor of view.wgsl
pull_requests: [20313]
---

All functions in view_transformations.wgsl have been replaced and deprecated.

To migrate, a straight-forward copy-paste inlining of the deprecated function's new body suffices, as they all now call the new api internally.

For example, if you had before:

```wgsl
#import bevy_pbr::view_transformations

let world_pos = view_transformations::position_view_to_world(view_pos);
```

Now it would be:

```wgsl
#import bevy_render::view

let world_pos = view::position_view_to_world(view_pos, view_bindings::view.world_from_view);
```

This was done to make it possible to pass in custom view bindings, and allow code reuse.

`view_transformations.wgsl` will be deleted in 0.18.
