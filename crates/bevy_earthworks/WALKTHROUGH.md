# bevy_earthworks Walkthrough

A guide to using the bevy_earthworks plugin for volumetric voxel terrain and construction machine simulation.

## 1. Running the Interactive Example

The quickest way to test the plugin is to run the interactive example:

```bash
cd /path/to/bevy
cargo run -p bevy_earthworks --example interactive
```

This will launch a 3D scene with:
- A 48x16x48 voxel terrain with a hill in the center
- Two construction machines (excavator and dozer)
- An orbit camera for navigation

## 2. Controls

### Camera
- **Right mouse drag** - Orbit/rotate camera around the scene
- **Middle mouse drag** - Pan camera
- **Scroll wheel** - Zoom in/out
- **W/A/S/D** - Move camera target forward/left/back/right
- **Q/E** - Move camera target down/up
- **Shift** - Move faster

### Playback
- **Space** - Toggle play/pause
- **R** - Reset playback to start
- **1** - Set speed to 0.5x
- **2** - Set speed to 1.0x (normal)
- **3** - Set speed to 2.0x
- **4** - Set speed to 4.0x

## 3. Creating Your Own Scene

Here's a minimal example showing how to use the plugin in your own app:

```rust
use bevy::prelude::*;
use bevy_earthworks::prelude::*;
use bevy_earthworks::camera::OrbitCamera;
use bevy_earthworks::terrain::{Chunk, ChunkCoord, DirtyChunk, MaterialId, Voxel};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EarthworksPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut terrain: ResMut<VoxelTerrain>,
) {
    // Camera with orbit controls
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(30.0, 20.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera::new().with_target(Vec3::ZERO).with_distance(40.0),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create a simple terrain chunk
    let mut chunk = Chunk::new();
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..4 {
                chunk.set(x, y, z, Voxel::solid(MaterialId::Dirt));
            }
        }
    }

    let coord = ChunkCoord::new(0, 0, 0);
    let entity = commands.spawn((chunk, coord, DirtyChunk)).id();
    terrain.set_chunk_entity(coord, entity);
}
```

## 4. Spawning Machines

```rust
use bevy_earthworks::machines::*;

// Spawn an excavator
commands.spawn((
    Mesh3d(your_mesh),
    MeshMaterial3d(your_material),
    Transform::from_xyz(5.0, 4.0, 5.0),
    Machine {
        id: "excavator-1".to_string(),
        machine_type: MachineType::Excavator,
        capacity: 10.0,
        current_load: 0.0,
        fuel: 1.0,
    },
    WorkEnvelope::Toroidal {
        inner_radius: 3.0,
        outer_radius: 8.0,
        min_height: -3.0,
        max_height: 2.0,
    },
    Mobility::default(),
    MachineActivity::Idle,
));
```

## 5. Loading Execution Plans

Plans are JSON files describing machine operations. A sample plan is at:
`crates/bevy_earthworks/assets/plans/demo.plan.json`

### Plan Structure

```json
{
  "version": "1.0",
  "metadata": {
    "name": "Plan Name",
    "duration": 60.0,
    "total_volume": 100.0
  },
  "site": {
    "bounds": [48, 16, 48],
    "voxel_size": 1.0
  },
  "machines": [
    {
      "id": "excavator-1",
      "machine_type": "Excavator",
      "initial_position": [5.0, 4.0, 5.0]
    }
  ],
  "steps": [
    {
      "timestamp": 0.0,
      "machine_id": "excavator-1",
      "action": {
        "type": "MoveTo",
        "target": [15.0, 4.0, 15.0]
      },
      "duration": 5.0
    },
    {
      "timestamp": 5.0,
      "machine_id": "excavator-1",
      "action": {
        "type": "Excavate",
        "target": [20.0, 6.0, 20.0],
        "volume": 10.0
      },
      "duration": 8.0
    }
  ]
}
```

### Available Actions

- **MoveTo** - Move machine to a position
- **Excavate** - Dig material at a target location
- **Dump** - Deposit carried material
- **Push** - Push material in a direction (dozers)
- **Idle** - Wait for a duration
- **WaitFor** - Wait for another machine to complete a step

## 6. Available Materials

| Material | Description |
|----------|-------------|
| `MaterialId::Air` | Empty/transparent |
| `MaterialId::Dirt` | Brown dirt |
| `MaterialId::Clay` | Tan clay |
| `MaterialId::Rock` | Gray rock |
| `MaterialId::Topsoil` | Dark brown topsoil |
| `MaterialId::Gravel` | Gray gravel |
| `MaterialId::Sand` | Beige sand |
| `MaterialId::Water` | Blue water (semi-transparent) |

## 7. Machine Types and Work Envelopes

### Excavators - Toroidal (donut-shaped) work area

```rust
WorkEnvelope::Toroidal {
    inner_radius: 3.0,   // Can't dig too close
    outer_radius: 8.0,   // Maximum reach
    min_height: -4.0,    // Can dig below
    max_height: 3.0,     // And above
}
```

### Dozers - Rectangular area in front

```rust
WorkEnvelope::Rectangular {
    width: 3.0,   // Blade width
    depth: 4.0,   // Push distance
    height: 1.0,  // Blade height
}
```

### Loaders - Arc in front

```rust
WorkEnvelope::Arc {
    radius: 6.0,
    angle: std::f32::consts::PI / 2.0,  // 90 degrees
    min_height: 0.0,
    max_height: 4.0,
}
```

## 8. Controlling Playback Programmatically

```rust
fn control_playback(mut playback: ResMut<PlanPlayback>) {
    playback.play();           // Start playback
    playback.pause();          // Pause
    playback.toggle();         // Toggle play/pause
    playback.reset();          // Reset to start
    playback.seek(10.0);       // Jump to 10 seconds
    playback.set_speed(2.0);   // 2x speed

    // Query state
    let time = playback.current_time();
    let duration = playback.duration();
    let progress = playback.progress();  // 0.0 to 1.0
    let is_playing = playback.is_playing();
}
```

## 9. Terrain Operations

```rust
use bevy_earthworks::terrain::operations::*;

fn modify_terrain(mut terrain: ResMut<VoxelTerrain>, mut commands: Commands) {
    // Excavate a sphere of material
    excavate_sphere(
        &mut terrain,
        &mut commands,
        Vec3::new(10.0, 5.0, 10.0),  // center
        3.0,                          // radius
    );

    // Fill a region with material
    fill_box(
        &mut terrain,
        &mut commands,
        IVec3::new(0, 0, 0),      // min corner
        IVec3::new(5, 3, 5),      // max corner
        MaterialId::Gravel,
    );
}
```

## 10. Running the Examples

```bash
# Minimal example (simpler)
cargo run -p bevy_earthworks --example minimal

# Interactive example (full-featured)
cargo run -p bevy_earthworks --example interactive
```

## 11. Plugin Configuration

```rust
use bevy_earthworks::config::EarthworksConfig;

fn setup(mut config: ResMut<EarthworksConfig>) {
    config.voxel_size = 1.0;           // Size of each voxel in meters
    config.chunk_render_distance = 4;   // Chunks to render around camera
    config.enable_shadows = true;
    config.enable_ambient_occlusion = false;
}
```

## Architecture Overview

```
bevy_earthworks/
├── terrain/       # Voxel terrain system
│   ├── voxel.rs   # Voxel and VoxelTerrain
│   ├── chunk.rs   # Chunk storage
│   ├── meshing.rs # Mesh generation (simple & greedy)
│   └── operations.rs # Terrain modification
├── machines/      # Construction machines
│   ├── components.rs # Machine, WorkEnvelope, etc.
│   └── animation.rs  # Movement and action animation
├── plan/          # Execution plans
│   ├── schema.rs  # Plan data structures
│   ├── loader.rs  # JSON asset loader
│   ├── playback.rs # Playback control
│   └── executor.rs # Step execution
├── camera/        # Orbit camera
├── scoring/       # Progress tracking
├── ui/            # Timeline UI
└── effects/       # Visual effects (future)
```
