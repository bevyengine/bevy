# Usage Guide: Bevy Remote Inspector

## Quick Start

### 1. Run the Target Application

First, start the example target application that includes bevy_remote:

```bash
# From the bevy_remote_inspector directory
cargo run --example target_app --features bevy_remote
```

This will:
- Start a Bevy application with bevy_remote enabled on `localhost:15702`
- Create various entities (Player, Enemies, Items, etc.)
- Animate entities and update their components in real-time
- Show console output indicating the server is ready

Expected output:
```
Starting target application for remote inspector
bevy_remote will be available at http://localhost:15702
Start the remote inspector to connect and view entities
Setting up demo scene with entities...
Demo scene setup complete!
   - 1 Camera
   - 1 Player
   - 3 Enemies
   - 4 Items
   - 5 Basic entities
Total: 14 entities created
```

### 2. Run the Remote Inspector

In a separate terminal, start the remote inspector:

```bash
# From the bevy_remote_inspector directory
cargo run --bin bevy_remote_inspector
```

The inspector will:
- Attempt to connect to `http://localhost:15702`
- Display connection status in the UI
- Show a live entity list when connected
- Allow selection and inspection of individual entities

## What You'll See

### Target Application Features
- **Visual 3D scene**: Green player cube, red enemy cubes, blue item spheres
- **Dynamic entities**: Player, enemies, and items with custom components
- **Live updates**: Entities move around, player stats change over time
- **Periodic spawning**: New colorful items appear every 10 seconds with unique colors
- **Component variety**: Transform, Player, Enemy, Item, Name components
- **Lighting**: Directional light for proper 3D visualization

### Inspector Features
- **Entity List**: Shows all entities with names (left panel)
- **Component Viewer**: Displays components for selected entity (right panel)
- **Live Updates**: Data refreshes automatically as target app changes
- **Connection Status**: Shows connection state in bottom-right corner

## Custom Components

The target app includes these custom components you can inspect:

### Player Component
```rust
struct Player {
    pub health: i32,     // Regenerates over time
    pub speed: f32,      // Oscillates with sine wave
    pub level: u32,      // Increases every 30 seconds
}
```

### Enemy Component
```rust
struct Enemy {
    pub damage: i32,     // Static damage value
    pub health: i32,     // Static health value
    pub ai_type: String, // "Aggressive", "Defensive", or "Patrol"
}
```

### Item Component
```rust
struct Item {
    pub name: String,    // Item name
    pub value: i32,      // Item value
    pub stackable: bool, // Whether item can stack
}
```

## Troubleshooting

### "Waiting for HTTP connection"
- Ensure the target application is running first
- Check that port 15702 is not blocked
- Verify bevy_remote feature is enabled

### "No entities visible"
- The target app should show entity count in console
- Try selecting different entities in the list
- Check that components are registered with `register_type::<T>()`

### Performance Issues
- The target app intentionally creates animations and updates
- This demonstrates live data changes in the inspector
- You can modify the example to reduce update frequency

## Extending the Example

To add your own components:

1. Define component with `#[derive(Component, Reflect)]`
2. Add `#[reflect(Component)]` attribute  
3. Register with `.register_type::<YourComponent>()`
4. Spawn entities with your component

The inspector will automatically display any registered, reflected components.