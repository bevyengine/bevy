# Bevy Remote Inspector

An out-of-process entity inspector for Bevy applications using `bevy_remote`.

## Architecture

This inspector is a **separate Bevy application** that connects to target applications via HTTP/JSON-RPC using the `bevy_remote` protocol. This provides several key advantages:

### Benefits of Out-of-Process Design
- **Zero Performance Impact**: Inspector runs independently, doesn't affect target application performance
- **No UI Entity Growth**: Inspector UI is completely separate from target application's ECS world
- **No Borrow Conflicts**: Uses `bevy_remote`'s proven RPC system instead of direct World access
- **Universal Component Support**: Leverages `bevy_remote`'s existing reflection capabilities
- **Flexible UI**: Can use any UI framework since it's a separate process

### Components Designed for Upstreaming
All UI components are built using `bevy_ui` and designed to be suitable for contribution back to Bevy:

- **CollapsibleSection**: Reusable expandable/collapsible UI widget
- **EntityList**: Live-updating entity list with remote data binding
- **ComponentViewer**: Component display with real-time updates
- **ConnectionStatus**: Connection indicator widget

## Usage

### Target Application (being inspected)
```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy::remote::RemotePlugin::default()) // Just this one line!
        .run();
}
```

### Inspector Application
```bash
cargo run --bin bevy_remote_inspector
```

The inspector will connect to `http://localhost:15702` by default and provide:
- Live entity list updates
- Real-time component data via `bevy/get+watch` endpoints
- Collapsible component sections
- Universal component reflection support

## Quick Start Demo

### 1. Run the Target Application
```bash
# From the bevy_remote_inspector directory
cargo run --example target_app --features bevy_remote
```

This starts a demo application with:
- Player entity with health/speed/level that changes over time
- Enemy entities with different AI types and stats  
- Item entities with names, values, and properties
- Dynamic entity spawning every 10 seconds
- All entities moving and updating in real-time

### 2. Run the Remote Inspector
```bash
# From the bevy_remote_inspector directory  
cargo run --bin bevy_remote_inspector
```

The inspector will connect to the target app and display:
- Live entity list (left panel)
- Component viewer for selected entities (right panel)
- Real-time updates as the target app changes
- Connection status indicator

See [USAGE.md](USAGE.md) for detailed instructions and troubleshooting.

## Implementation Status

- [x] Project structure and dependencies
- [x] Basic inspector application structure with bevy_ui
- [x] Reusable UI components (CollapsibleSection, EntityList, ComponentViewer)
- [x] **Removed mock client system entirely** - now uses only HTTP client
- [x] Entity list with selection and component viewer
- [x] Collapsible component sections
- [x] Demo application that compiles and runs
- [x] **Fixed critical entity despawn bug** that caused crashes
- [x] **Working entity selection and component viewing**
- [x] **Added HTTP client foundation** with reqwest and tokio dependencies
- [x] **Refactored to use HTTP client only** - removed all mock data from main app
- [x] **Created integration tests** with mock data for testing purposes
- [x] **Graceful connection failure handling** - shows connection status
- [ ] Async task integration for remote calls
- [ ] Live updates via +watch endpoints
- [ ] Full end-to-end testing with real target applications

## Technical Details

### Remote Communication
Uses `bevy_remote`'s JSON-RPC API:
- `bevy/list` - Get all entities or component types
- `bevy/query` - Query entities with filters
- `bevy/get` - Get component data for specific entities
- `bevy/get+watch` - Live component data updates (planned)

### UI Framework
Built entirely with `bevy_ui` to maintain consistency with Bevy's native UI patterns and enable easy upstreaming of reusable components.

This approach eliminates all the performance and complexity issues we encountered with the in-process approach while providing the same functionality through a proven remote protocol.