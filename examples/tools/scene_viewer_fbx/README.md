# FBX Scene Viewer

A comprehensive FBX scene viewer built with Bevy, designed specifically for viewing and inspecting FBX 3D models with enhanced debugging capabilities.

## Features

- **Complete FBX Support**: Load and view FBX files with meshes, materials, textures, and node hierarchies
- **Enhanced Material Inspection**: Detailed material property display and texture debugging
- **Interactive Controls**: Camera movement, lighting controls, and scene navigation
- **Real-time Information**: Live FBX asset statistics and material information
- **Professional Debugging**: Bounding box visualization and material debug modes

## Usage

### Basic Usage

```bash
# View a specific FBX file
cargo run --release --example scene_viewer_fbx --features="fbx" path/to/your/model.fbx

# View the default cube model
cargo run --release --example scene_viewer_fbx --features="fbx"

# With additional rendering options
cargo run --release --example scene_viewer_fbx --features="fbx" --depth-prepass --deferred path/to/model.fbx
```

### Command Line Options

- `--depth-prepass`: Enable depth prepass for better performance
- `--occlusion-culling`: Enable occlusion culling (requires depth prepass)
- `--deferred`: Enable deferred shading
- `--add-light`: Force spawn a directional light even if the scene has lights

## Controls

### Camera Controls
- **WASD**: Move camera
- **Mouse**: Look around
- **Shift**: Run (faster movement)
- **C**: Cycle through cameras (scene cameras + controller camera)

### Lighting Controls
- **L**: Toggle light animation (rotate directional light)
- **U**: Toggle shadows on/off

### Debug Controls
- **B**: Toggle bounding box visualization
- **M**: Toggle material debug information display
- **I**: Print detailed FBX asset information to console

### Material Debug Information

When pressing **I**, the viewer will display:
- Number of meshes, materials, nodes, skins, and animations
- Detailed material properties (base color, metallic, roughness)
- Texture information (which textures are applied)
- Scene statistics

Example output:
```
=== FBX Asset Information ===
Meshes: 17
Materials: 5
Nodes: 22
Skins: 0
Animation clips: 0

=== Material Details ===
Material 0: base_color=Srgba(red: 0.8, green: 0.8, blue: 0.8, alpha: 1.0), metallic=0, roughness=0.5
  - Has base color texture
  - Has normal map
  - Has metallic/roughness texture

=== Scene Statistics ===
Total mesh entities: 17
Total material entities: 17
```

## FBX Features Supported

### ✅ Fully Supported
- **Mesh Geometry**: Vertices, normals, UVs, indices
- **Materials**: PBR properties with automatic texture application
- **Textures**: All common texture types automatically mapped to StandardMaterial
- **Node Hierarchy**: Complete scene graph with transforms
- **Skinning Data**: Joint weights and inverse bind matrices

### 🔧 Enhanced Features
- **Automatic Texture Mapping**: FBX textures automatically applied to Bevy materials
- **Material Property Extraction**: Base color, metallic, roughness, emission
- **Real-time Debugging**: Live material and texture information
- **Professional Inspection**: Detailed asset statistics and debugging tools

### ⚠️ Framework Ready
- **Animations**: Complete framework in place, temporarily disabled

## Comparison with GLTF Scene Viewer

| Feature | GLTF Scene Viewer | FBX Scene Viewer |
|---------|-------------------|------------------|
| File Format | GLTF/GLB | FBX |
| Material Inspection | Basic | Enhanced with debug info |
| Texture Debugging | Limited | Comprehensive |
| Asset Information | Scene-based | Detailed FBX-specific |
| Animation Support | Full | Framework ready |
| Real-time Debug | Basic | Professional-grade |

## Technical Details

### Architecture
- Built on Bevy's asset system with FBX-specific enhancements
- Uses ufbx library for robust FBX parsing
- Automatic texture-to-material mapping
- Professional debugging and inspection tools

### Performance
- Efficient mesh loading and rendering
- Optional depth prepass and occlusion culling
- Deferred shading support for complex scenes
- Real-time material property inspection

### Debugging Capabilities
- Live FBX asset statistics
- Material property inspection
- Texture mapping verification
- Bounding box visualization
- Scene hierarchy analysis

## Examples

### Viewing Different FBX Files
```bash
# Simple cube model
cargo run --example scene_viewer_fbx --features="fbx" assets/models/cube/cube.fbx

# Animated model
cargo run --example scene_viewer_fbx --features="fbx" assets/models/cube_anim.fbx

# Complex scene with materials
cargo run --example scene_viewer_fbx --features="fbx" assets/models/instanced_materials.fbx
```

### Performance Testing
```bash
# High-performance mode with all optimizations
cargo run --release --example scene_viewer_fbx --features="fbx" --depth-prepass --occlusion-culling --deferred large_model.fbx
```

## Troubleshooting

### Common Issues
1. **FBX file not loading**: Ensure the file path is correct and the FBX file is valid
2. **Missing textures**: Check that texture files are in the same directory or use absolute paths
3. **Performance issues**: Try enabling `--depth-prepass` and `--deferred` for complex scenes

### Debug Information
Use the **I** key to get detailed information about the loaded FBX file, including:
- Asset counts and statistics
- Material properties and texture mappings
- Scene hierarchy information
- Performance metrics

This tool is perfect for FBX asset inspection, material debugging, and scene analysis in Bevy applications.
