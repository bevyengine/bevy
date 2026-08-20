---
title: Sprite materials
authors: ["@cookie1170"]
pull_requests: [25415]
---

**TODO: Add recording showcasing the `sprite_material` example**

Until now, Bevy's sprite renderer has been lacking a major feature: the ability to extend it with custom shaders!
With this release, it's now possible to create custom materials for sprites by implementing the `MaterialExtension2d` trait,
inserting the `SpriteMaterial` component and adding the `SpriteMaterialPlugin` to your app.

The shader can use functions exported from `bevy_sprite_render::sprite_mesh::functions`, including:
```wesl
// Samples the sprite's final color, including the tint and alpha discard, at a given UV.
fn sample_final_color(uv: vec2<f32>, instance_index: u32) -> vec4<f32>;

// Samples the sprite's texture without tint and alpha discard at a given UV.
fn sample_sprite_texture(uv: vec2<f32>, instance_index: u32) -> vec4<f32>;

// Applies tint and alpha discard to the sprite's color.
fn get_final_color(sprite_color: vec4<f32>, instance_index: u32) -> vec4<f32>;
```

Check out the `sprite_material` example to see it in action!
