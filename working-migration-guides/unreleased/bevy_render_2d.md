# Create `bevy_render_2d` crate

prs = [[18467](https://github.com/bevyengine/bevy/pull/18467)]

Extract from `bevy_sprite` code that is not exclusive to sprites and move it to
new crate `bevy_render_2d`. New locations for symbols are as follows:

## Struct

Struct | `0.16` Path | `0.17` Path
--- | --- | ---
`MeshMaterial2d` | | `bevy_render_2d::material`
`AlphaMode2d` | | `bevy_render_2d::material`
`Material2dKey` | | `bevy_render_2d::material::key`

## Traits

Trait | `0.16` Path | `0.17` Path
--- | --- | ---
`Material2d` | | `bevy_render_2d::material`

## Plugins

Trait | `0.16` Path | `0.17` Path
--- | --- | ---
`Material2dPlugin` | | `bevy_render_2d::material::Plugin`

## Prelude

`bevy_render_2d`'s prelude contains:
* `Material2d`
* `MeshMaterial2d`
* `AlphaMode2d`
