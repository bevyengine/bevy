---
title: "`SpriteMaterial` and `SpriteMaterialPlugin` rename"
pull_requests: [25415]
---

- `bevy_sprite_render::SpriteMaterial` has been renamed to `SpriteMeshMaterial`. Note that this type is rarely used outside of the sprite's implementation.
- `bevy_sprite_render::SpriteMaterialPlugin` has been renamed to `SpriteMeshMaterialPlugin`. Note that this plugin is usually added by the `SpriteMeshPlugin` or `SpriteRenderPlugin` instead.
