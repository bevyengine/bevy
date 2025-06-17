# Bevy FBX

A Bevy plugin for loading FBX files using the [ufbx](https://github.com/ufbx/ufbx) library.

## Features

- ✅ **Mesh Loading**: Load 3D meshes with vertices, normals, UVs, and indices
- ✅ **Material Support**: Basic PBR material loading with textures
- ✅ **Skinning Data**: Complete skinning support with joint weights and inverse bind matrices
- ✅ **Animation Framework**: Structure ready for animation clip loading
- ✅ **Node Hierarchy**: Basic scene graph support
- ⚠️ **Textures**: Loaded but not yet applied to materials
- ⚠️ **Animations**: Framework in place, needs ufbx animation API integration

## Usage

### Enable the Feature

FBX support is an optional feature in Bevy. Add it to your `Cargo.toml`:

```toml
[dependencies]
bevy = { version = "0.16", features = ["fbx"] }
```

### Loading FBX Files

```rust
use bevy::prelude::*;
use bevy::fbx::FbxAssetLabel;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load an FBX file
    let fbx_handle: Handle<bevy::fbx::Fbx> = asset_server.load("models/my_model.fbx");

    // Spawn the FBX scene
    commands.spawn(SceneRoot(fbx_handle));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}
```

### Accessing Individual Assets

```rust
use bevy::fbx::{Fbx, FbxAssetLabel};

fn access_fbx_assets(
    fbx_assets: Res<Assets<Fbx>>,
    fbx_handle: Handle<Fbx>,
) {
    if let Some(fbx) = fbx_assets.get(&fbx_handle) {
        // Access meshes
        for mesh_handle in &fbx.meshes {
            println!("Found mesh: {:?}", mesh_handle);
        }

        // Access materials
        for material_handle in &fbx.materials {
            println!("Found material: {:?}", material_handle);
        }

        // Access skins (for skeletal animation)
        for skin_handle in &fbx.skins {
            println!("Found skin: {:?}", skin_handle);
        }

        // Access animation clips
        for animation_handle in &fbx.animation_clips {
            println!("Found animation: {:?}", animation_handle);
        }
    }
}
```

## Asset Labels

You can load specific parts of an FBX file using asset labels:

```rust
// Load a specific mesh by index
let mesh: Handle<Mesh> = asset_server.load("model.fbx#Mesh0");

// Load a specific material by index
let material: Handle<StandardMaterial> = asset_server.load("model.fbx#Material0");

// Load a specific skin by index
let skin: Handle<bevy::fbx::FbxSkin> = asset_server.load("model.fbx#Skin0");

// Load a specific animation by index
let animation: Handle<AnimationClip> = asset_server.load("model.fbx#Animation0");
```

## Supported FBX Features

- **Geometry**: Triangulated meshes with positions, normals, UVs
- **Materials**: Basic PBR properties (base color, metallic, roughness)
- **Textures**: Diffuse, normal, metallic, roughness maps (loaded but not applied)
- **Skinning**: Joint weights, indices, and inverse bind matrices
- **Hierarchy**: Node transforms and basic parent-child relationships

## Limitations

- Animations are not yet fully implemented
- Complex material features are not supported
- Some FBX-specific features may not be available
- Large files may have performance implications

## Examples

See `examples/3d/load_fbx.rs` for a complete example of loading and displaying FBX files.

## Technical Details

This plugin uses the [ufbx](https://github.com/ufbx/ufbx) library, which provides:
- Fast and reliable FBX parsing
- Support for FBX versions 6.0 and later
- Memory-safe C API with Rust bindings
- Comprehensive geometry and animation support

The plugin follows Bevy's asset loading patterns and integrates seamlessly with the existing rendering pipeline.
