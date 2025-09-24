bevy_fbx — FBX loader for Bevy
================================

bevy_fbx is an FBX asset loader for Bevy built on top of the excellent
[ufbx](https://github.com/ufbx/ufbx) library (via the
[ufbx-rust](https://github.com/ufbx/ufbx-rust) bindings).

The goal is to provide a pragmatic, batteries‑included way to bring FBX scenes
into Bevy with sensible defaults and good round‑trip behavior with DCC tools.

Status: experimental but usable. The loader focuses on meshes, materials,
hierarchy, basic cameras/lights, skinning and T/R/S animations.


Highlights
----------
- Scenes: loads complete FBX scenes and exposes labeled sub‑assets (Meshes,
  Materials, Nodes, Skins, Animations, …).
- Meshes: vertex positions, normals, UVs, indices; multi‑material meshes are
  split into sub‑meshes per material group.
- Materials: converts common FBX Phong/PBR parameters into Bevy's
  `StandardMaterial`, including:
  - base color/metallic/roughness/emissive factors;
  - textures for base color, metallic/roughness (or either), emission, AO,
    normal (treated as linear);
  - alpha: Opaque, Blend or Mask (alpha‑cutoff) depending on opacity and maps;
  - double‑sided materials disable face culling;
  - UV transforms are applied using ufbx's UV→Texture transform matrix.
- Textures: respects FBX `wrap_u/v` (Repeat/Clamp). You can override or supply
  default samplers from Bevy.
- Hierarchy: reconstructs the FBX node tree; each mesh is attached as a child
  using `geometry_to_node` so placements match DCC.
- Cameras/Lights: creates Bevy camera/light components from FBX nodes (first
  camera is made active). Orthographic cameras currently use a perspective
  fallback.
- Skinning: adds `SkinnedMesh` with inverse bind poses and joint entity list;
  per‑vertex joint indices/weights are uploaded to the mesh.
- Animation: builds an `AnimationClip` + `AnimationGraph` from FBX layers.
  Supports both verbose (Lcl Translation/Rotation/Scaling) and short (T/R/S)
  property names for keyframes; auto‑plays the first clip.


Quick start
-----------

Enable the loader by turning on the `fbx` feature in your Bevy app, or by
depending on `bevy_fbx` explicitly.

Option A — via Bevy features (recommended):

```toml
[dependencies]
bevy = { version = "0.18", features = ["fbx"] }
```

Option B — direct crate dependency:

```toml
[dependencies]
bevy = "0.18"
bevy_fbx = "0.18"
```

Register the loader. If you use `DefaultPlugins` and enabled the `fbx` feature
on Bevy, the plugin is included automatically. Otherwise, add it manually:

```rust
use bevy::prelude::*;
use bevy_fbx::FbxPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FbxPlugin)
        .run();
}
```

Loading a scene:

```rust
use bevy::prelude::*;
use bevy::fbx::FbxAssetLabel; // re-exported when using Bevy with the `fbx` feature

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // Spawn the first scene contained in the FBX file
    commands.spawn(SceneRoot(
        assets.load(FbxAssetLabel::Scene(0).from_asset("models/thing.fbx")),
    ));
}
```

Loading specific sub‑assets:

```rust
use bevy::prelude::*;
use bevy::fbx::FbxAssetLabel;

fn load_parts(assets: Res<AssetServer>) {
    let mesh0: Handle<Mesh> = assets.load(FbxAssetLabel::Mesh(0).from_asset("models/thing.fbx"));
    let mat0: Handle<StandardMaterial> = assets.load(FbxAssetLabel::Material(0).from_asset("models/thing.fbx"));
    let scene0: Handle<Scene> = assets.load(FbxAssetLabel::Scene(0).from_asset("models/thing.fbx"));
}
```


Loader settings
---------------

`FbxLoaderSettings` lets you tweak what gets imported and how:

```rust
use bevy::prelude::*;
use bevy::fbx::{Fbx, FbxLoaderSettings};

fn setup(assets: Res<AssetServer>) {
    // Example: disable lights, convert coordinates, force a sampler
    let _fbx: Handle<Fbx> = assets.load_with_settings(
        "models/thing.fbx",
        |s: &mut FbxLoaderSettings| {
            s.load_lights = false;
            s.convert_coordinates = true; // flips Z to match Bevy (-Z forward)
            s.override_sampler = true;
        },
    );
}
```

Available fields (see the code for details):

- `load_meshes: RenderAssetUsages` — retain meshes in main/render worlds.
- `load_materials: RenderAssetUsages` — retain materials in main/render worlds.
- `load_cameras: bool` — spawn FBX cameras.
- `load_lights: bool` — spawn FBX lights.
- `include_source: bool` — kept for GLTF API parity (no effect with ufbx).
- `default_sampler: Option<ImageSamplerDescriptor>` — default texture sampler.
- `override_sampler: bool` — ignore FBX sampler data and use the default.
- `convert_coordinates: bool` — convert FBX coords to Bevy (flip Z).


Labeled sub‑assets
------------------

`FbxAssetLabel` covers the common pieces you may want to address directly:

- `Scene{N}`, `Mesh{N}`, `Material{N}`, `Animation{N}`, `AnimationGraph{N}`
- `AnimationStack{N}`, `Skeleton{N}`, `Node{N}`, `Skin{N}`
- `Light{N}`, `Camera{N}`, `Texture{N}`
- `DefaultScene`, `DefaultMaterial`, `RootNode`

You can also use raw strings (`"models/foo.fbx#Mesh0"`) if you prefer.


Examples in this repo
---------------------

- Load an FBX scene: `examples/3d/load_fbx.rs`
- Play an FBX animation: `examples/animation/animated_mesh_fbx.rs`
- Inspect any FBX on disk: `examples/tools/scene_viewer_fbx` (pass a path)

Run (requires the `fbx` Cargo feature):

```sh
cargo run --example load_fbx --features fbx
cargo run --example animated_mesh_fbx --features fbx
cargo run --example scene_viewer_fbx --features fbx -- /path/to/model.fbx
```


Differences vs `bevy_gltf`
--------------------------

- Source semantics
  - glTF has a strict PBR schema; FBX is looser. `bevy_fbx` maps the most common
    Phong/PBR fields pragmatically to `StandardMaterial`.
- Texture transforms
  - glTF uses `KHR_texture_transform` (per‑texture). `bevy_gltf` warns when
    transforms differ across maps. `bevy_fbx` applies ufbx's UV→Texture matrix
    and currently uses the base‑color transform for the material's global
    `uv_transform` (per‑material). Differing transforms on other maps are not
    yet handled.
- Samplers
  - glTF samplers (wrap + filter) are fully converted. `bevy_fbx` converts wrap
    (Repeat/Clamp) and leaves filter to Bevy defaults unless you override the
    sampler via settings.
- Cameras
  - glTF orthographic cameras map to Bevy orthographic. `bevy_fbx` currently
    uses a perspective fallback for orthographic FBX cameras.
- Animations
  - glTF channels (node‑targeted) are fully supported, including morph targets.
    `bevy_fbx` currently builds transform clips from T/R/S curves (supports both
    long `Lcl Translation/Rotation/Scaling` and short `T/R/S` names), assumes
    XYZ Euler for rotations, and does not yet import morph targets or other
    animated properties.
- Materials, double‑sided & culling
  - glTF double‑sided may flip culling when scale is inverted; `bevy_gltf`
    handles this. `bevy_fbx` disables culling for double‑sided and does not yet
    invert culling on negative scale paths.
- Coordinate conversion
  - Both loaders expose a `convert_coordinates` toggle; in `bevy_fbx` it flips Z
    to match Bevy's −Z forward.


FBX material → `StandardMaterial` mapping
----------------------------------------

| FBX (ufbx)                          | StandardMaterial field                     | Notes |
|-------------------------------------|--------------------------------------------|-------|
| `fbx.diffuse_color` or `pbr.base_color` | `base_color`                            | sRGB color |
| `pbr.metalness` (x)                 | `metallic`                                 | scalar |
| `pbr.roughness` (x)                 | `perceptual_roughness`                     | scalar |
| `fbx.emission_color` or `pbr.emission_color` | `emissive`                       | linear |
| `pbr.opacity` (value < 1.0)         | `alpha_mode`                               | `Blend` if no cutoff texture; may use `Mask(0.5)` if opacity texture present |
| `features.double_sided.enabled`     | `double_sided = true`, `cull_mode = None`  | disables face culling |
| Texture `DiffuseColor`/`BaseColor`  | `base_color_texture` + `uv_transform`      | `uv_transform` from ufbx UV→Texture matrix |
| Texture `NormalMap`                 | `normal_map_texture`                       | treated as linear |
| Texture `Metallic`                  | `metallic_roughness_texture`               | grayscale source; packed into Bevy MR texture slot |
| Texture `Roughness`                 | `metallic_roughness_texture` (if empty)    | fallback when Metallic not present |
| Texture `EmissiveColor`             | `emissive_texture`                         | |
| Texture `AmbientOcclusion`          | `occlusion_texture`                        | |
| Texture wrap U/V                    | sampler address mode U/V                   | Repeat / Clamp |

Notes:
- Bevy's MR texture expects metallic in B channel and roughness in G channel.
  FBX often provides separate grayscale maps; `bevy_fbx` assigns whichever is
  available to the MR slot (no packing). For best results author a packed map
  or rely on scalar metallic/roughness.
- Only the base‑color texture's transform is applied to `StandardMaterial`'s
  `uv_transform` (global for the material). If other textures need different
  transforms, they are currently ignored.


Roadmap
-------

- Materials
  - Per‑texture UV transforms (not just base‑color → `uv_transform`).
  - Wider PBR coverage (transmission, thickness/attenuation, IOR, clearcoat,
    anisotropy, specular textures) to parity with `bevy_gltf` where feasible.
- Animation
  - Connect curves to nodes via FBX connections/DAG rather than name fallbacks.
  - Respect FBX rotation orders and pre/post/adjust transforms during baking.
  - Morph target (blend shape) animation.
- Geometry & Skinning
  - Invert culling on negative scale paths (material copies) similar to
    `bevy_gltf`.
  - Additional validation for multi‑cluster/weight normalization edge cases.
- IO & Performance
  - Streaming/async‑friendly texture decode path; improved error messages.
  - Import metrics & debug visualization toggles.
- Tooling & Docs
  - Expand `scene_viewer_fbx` for material/channel inspection.
  - More examples and tests; deeper docs on coordinate conversion and UVs.


Coordinate systems
------------------

FBX is typically right‑handed with +Z forward, +Y up. Bevy uses +Y up and −Z
forward. Set `FbxLoaderSettings::convert_coordinates = true` if you want the
loader to flip Z for you. UV transforms are applied using ufbx's
`uv_to_texture` matrix so what you see matches DCC texture placement.


Limitations & notes
-------------------

- Material coverage is pragmatic: not every FBX/PBR parameter is mapped.
  (E.g. AO strength, some extensions, orthographic cameras use a perspective
  fallback.) Normal maps are treated as linear data.
- Animations currently parse T/R/S curves (both long and short property names)
  and assume XYZ Euler order when converting to quaternions. Other animated
  properties and morph targets are not yet supported.
- The loader splits meshes per material group; very large meshes with many
  materials will produce multiple sub‑meshes.


Troubleshooting
---------------

- “The model is tiny / I can't see it”: move the camera closer or scale the
  spawned `SceneRoot` entity for your preview use‑case.
- “Textures don't show up”: make sure texture files are available next to the
  FBX or using absolute paths referenced by the file.
- “Double‑sided materials don't look right”: double‑sided disables face culling;
  verify your winding order and opacity mode.


License
-------

Dual‑licensed under MIT or Apache‑2.0, at your option.


Acknowledgements
----------------

This crate is powered by the amazing work on ufbx and ufbx‑rust.
