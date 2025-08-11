# Bevy Entity Inspector

The Bevy Entity Inspector is a powerful debugging tool that allows you to inspect and monitor entities and components in real-time, both locally and remotely. It provides two distinct modes of operation:

1. **Local Inspector** - An embedded, in-game inspector overlay
2. **Remote Inspector** - An external application that connects to your game via `bevy_remote`

## Local Inspector (In-Game Overlay)

The local inspector provides an in-game overlay that can be toggled on/off during development.

### Setup

Add the `InspectorPlugin` to your application:

```rust
use bevy::dev_tools::inspector::InspectorPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin::debug()) // Use debug preset with F11 toggle
        .run();
}
```

### Usage

- Press **F11** to toggle the inspector overlay
- Browse entities in the left panel
- Click on entities to view their components in the right panel
- Component values update in real-time

### Example

Run the local inspector example:

```bash
cargo run --example inspector --features="bevy_dev_tools"
```

## Remote Inspector (External Application)

The remote inspector runs as a separate application that connects to your game over HTTP using the `bevy_remote` protocol. This is particularly useful for:

- Inspecting headless applications
- Debugging without UI overlay interference  
- External tooling and automation
- Multi-monitor setups

### Setup

#### Target Application (Your Game)

Add `bevy_remote` plugins to enable external connections:

```rust
use bevy::prelude::*;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RemotePlugin::default())        // Enable JSON-RPC
        .add_plugins(RemoteHttpPlugin::default())    // Enable HTTP transport
        // Your game systems here...
        .run();
}
```

#### Inspector Application

Create a separate inspector app:

```rust
use bevy::prelude::*;
use bevy::dev_tools::inspector::InspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin) // Remote inspector (no debug preset)
        .run();
}
```

### Usage

1. **Start your target application** with `bevy_remote` enabled:
   ```bash
   cargo run --example server --features="bevy_remote"
   ```

2. **Start the inspector** in a separate terminal/window:
   ```bash
   cargo run --example entity_inspector_minimal --features="bevy_dev_tools"
   ```

The inspector will automatically connect to `localhost:15702` and display your entities.

### Features

- **Real-time Updates**: Component values update live as they change
- **Interactive UI**: Click to select entities, text selection with copy/paste
- **Connection Resilience**: Auto-retry logic handles connection failures gracefully  
- **Performance**: Virtual scrolling efficiently handles large numbers of entities
- **All Components**: Automatically discovers and displays all component types

### Connection Details

- **Default Address**: `localhost:15702`
- **Protocol**: HTTP with JSON-RPC 2.0
- **Endpoints**: 
  - `/health` - Connection health check
  - `/jsonrpc` - Main JSON-RPC interface

## Component Registration

For components to be visible in the inspector, they must implement `Reflect`:

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct Player {
    health: i32,
    speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<Player>() // Required for reflection
        .run();
}
```

## Troubleshooting

### Remote Inspector Issues

**Inspector shows "Awaiting connection":**
- Ensure target app is running with `bevy_remote` plugins enabled
- Verify target app is listening on port 15702
- Check firewall/network connectivity

**Components not visible:**
- Ensure components implement `Reflect` 
- Register component types with `.register_type::<YourComponent>()`
- Verify components implement `Serialize`/`Deserialize` for remote inspection

**Connection drops frequently:**
- Check target application stability
- Monitor network connectivity
- The inspector will automatically retry connections

### Local Inspector Issues

**F11 doesn't toggle inspector:**
- Ensure you're using `InspectorPlugin::debug()` not `InspectorPlugin`
- Check if another system is handling F11 key input

**UI elements not visible:**
- Verify UI camera is present in your scene
- Check for UI layer conflicts

## Architecture

The inspector uses several key components:

- **HTTP Client** (`crates/bevy_dev_tools/src/inspector/http_client.rs`): Manages remote connections and JSON-RPC communication
- **UI Components** (`crates/bevy_dev_tools/src/inspector/ui/`): Entity list, component viewer, connection status
- **Virtual Scrolling**: Efficient rendering for large entity lists
- **Live Updates**: Real-time component value streaming

## Examples

| Example | Purpose | Command |
|---------|---------|---------|
| `inspector` | Local in-game overlay | `cargo run --example inspector --features="bevy_dev_tools"` |
| `server` | Target app for remote inspection | `cargo run --example server --features="bevy_remote"` |
| `entity_inspector_minimal` | Remote inspector client | `cargo run --example entity_inspector_minimal --features="bevy_dev_tools"` |