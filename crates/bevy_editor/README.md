# Bevy Editor

A modern inspector and editor for Bevy applications, designed to provide real-time introspection and editing capabilities.

![Bevy Editor](https://img.shields.io/badge/status-in_development-yellow.svg)
![Bevy](https://img.shields.io/badge/bevy-0.15-blue.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)

## Features

### Current (v0.1)
- **Real-time Connection**: HTTP client integration with `bevy_remote` protocol
- **Entity Inspection**: Browse and select entities in a clean, modern interface
- **Component Viewing**: Structured component display with hierarchical field breakdown
- **Smart Type Recognition**: Specialized formatting for Bevy types (Vec2/Vec3/Quat, Colors, Entity IDs)
- **Connection Status**: Live connection monitoring with visual status indicators
- **Modern UI**: Dark theme with professional styling and responsive design
- **Event-driven Architecture**: Built on Bevy's observer system for optimal performance
- **Expandable Structures**: Smart component exploration with keyboard shortcuts (E/T/C keys) for any component type
- **Mouse Wheel Scrolling**: Smooth scrolling through entity lists with optimized sensitivity
- **Dynamic Expansion**: Real-time [+]/[-] indicators based on current expansion state

### In Development
- **Component Editing**: Real-time component value modification
- **Entity Management**: Create, delete, and clone entities
- **Search & Filter**: Advanced filtering and search capabilities
- **Hierarchical Views**: Tree-based entity and component organization
- **System Inspector**: Monitor and control system execution
- **Data Persistence**: Save and load entity configurations

## Quick Start

### Prerequisites
- Rust 1.70+ with Cargo
- Bevy 0.15+
- A Bevy application with `bevy_remote` enabled

### Installation

1. **Add to your Bevy project**:
```toml
[dependencies]
bevy_editor = { path = "path/to/bevy_editor" }
```

2. **Enable bevy_remote in your target application**:
```rust
use bevy::prelude::*;
use bevy_remote::RemotePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RemotePlugin::default())
        .run();
}
```

3. **Run the inspector**:
```bash
cargo run --example inspector --package bevy_editor
```

4. **Start your Bevy application** (the one you want to inspect)

The inspector will automatically connect to `http://127.0.0.1:15702` and begin displaying entities.

## Usage

### Basic Workflow
1. **Launch Inspector**: Run the editor example
2. **Start Target App**: Launch your Bevy application with `bevy_remote` enabled
3. **Browse Entities**: Click on entities in the left panel to view their components
4. **Inspect Components**: View detailed component data in the right panel
5. **Monitor Status**: Check the connection status in the top status bar

### Connection Configuration
The inspector connects to `bevy_remote` servers. The default endpoint is:
- **URL**: `http://127.0.0.1:15702`
- **Protocol**: JSON-RPC 2.0 over HTTP
- **Polling**: 1-second intervals

### Component Display
Components are displayed in a structured, hierarchical format:
```
[Component] Transform
  [+] translation: (0.000, 0.000, 0.000)
    x: 0.000
    y: 0.000
    z: 0.000
  [+] rotation: (0.000, 0.000, 0.000, 1.000)
    x: 0.000
    y: 0.000
    z: 0.000
    w: 1.000
  [+] scale: (1.000, 1.000, 1.000)
    x: 1.000
    y: 1.000
    z: 1.000

[Component] Visibility
  inherited: true
```

### Usage

Once running, the editor provides:

1. **Entity Selection**: Click on entities in the left panel to select them
2. **Component Inspection**: Selected entity components appear in the right panel with hierarchical display
3. **Interactive Expansion**: 
   - Press `E` to expand common component fields automatically (translation, rotation, scale, position, velocity, color, etc.)
   - Press `T` to toggle Transform component fields specifically
   - Press `C` to collapse all expanded fields
4. **Mouse Navigation**: Use mouse wheel to scroll through the entity list
5. **Connection Status**: Monitor connection status in the top status bar

## Development Roadmap

### Widget System (✅ COMPLETED - v0.1)
**Goal**: Create modular widget system for eventual bevy_feathers extraction

- [x] **ScrollableContainer**: Basic scrollable container with mouse wheel support
- [x] **BasicPanel**: Simple panel container with title and configuration
- [x] **ExpansionButton**: Interactive expansion buttons for hierarchical content
- [x] **Theme Integration**: Basic theme system with consistent styling
- [x] **Plugin Architecture**: Each widget has its own plugin system
- [x] **Documentation**: Comprehensive documentation for PR readiness
- [x] **Clean Compilation**: All compilation errors resolved, minimal warnings

**Implementation Details**:
- All widgets designed for bevy_feathers extraction
- Minimal dependencies on core Bevy systems
- Plugin-based architecture for modularity
- Consistent API patterns across widgets
- Theme integration for styling consistency

See `WIDGETS.md` for detailed widget system documentation.

### Phase 1: Enhanced Component Display
**Goal**: Transform raw JSON into structured, readable component fields

- [x] **Structured Parsing**: Parse JSON into typed fields with proper formatting
- [x] **Type-aware Display**: Specialized rendering for common Bevy types (Vec3, Quat, Color, etc.)
- [x] **Expandable Structures**: Foundation with [+] indicators for collapsible nested objects and arrays
- [x] **Value Formatting**: Human-readable formatting for different data types (Entity IDs, truncated strings, precision-controlled numbers)
- [x] **Hierarchical Layout**: Proper indentation and nested structure display
- [x] **Interactive Expansion**: Keyboard-based expansion system (E to expand, C to collapse) with state tracking
- [x] **Mouse Wheel Scrolling**: Scrollable entity list with mouse wheel support
- [ ] **Clickable Expansion**: Replace keyboard shortcuts with clickable [+]/[-] buttons
- [ ] **Visual Polish**: Enhanced styling with consistent spacing and visual hierarchy
- [ ] **Advanced Type Support**: Support for more complex Bevy types (Asset handles, Entity references, etc.)

#### Phase 1 - Remaining Implementation Details

**Interactive Expansion System** ( **IMPLEMENTED**):
-  Add expansion state tracking with `ComponentDisplayState` resource
-  Update `format_field_recursive()` to check expansion state before showing children
-  Dynamic [+]/[-] indicators based on expansion state
-  Smart keyboard shortcuts: 'E' for common fields, 'T' for Transform, 'C' to collapse all
-  Generic field detection for any component type (not just Transform)
- **Next**: Replace keyboard shortcuts with clickable UI elements

**Mouse Wheel Scrolling** ( **IMPLEMENTED**):
-  Added `ScrollableArea` component for marking scrollable UI elements  
-  Mouse wheel scroll handler for entity list navigation
-  Smooth scrolling with optimal sensitivity (5px per wheel unit)

**Visual Polish** (Next Priority):
- Consistent color coding for different value types (numbers, strings, booleans)
- Improved spacing and visual hierarchy
- Better visual distinction between expandable and non-expandable items
- Add subtle hover effects for better interactivity

**Advanced Type Support**:
- Asset handle detection and formatting (e.g., "Handle<Mesh>", "Handle<Image>")
- Entity reference formatting with clickable navigation
- Support for Bevy's built-in components (Camera, Mesh, Material handles)
- Custom type registration system for user-defined components

### Phase 2: Interactive Component Editing
**Goal**: Enable real-time modification of component values

- [ ] **Input Fields**: Type-appropriate input controls (sliders, text fields, checkboxes)
- [ ] **Real-time Updates**: Live synchronization with the target application
- [ ] **Validation System**: Client-side validation before sending changes
- [ ] **Error Handling**: Graceful handling of invalid values and server errors
- [ ] **Undo/Redo**: Basic change history and rollback capabilities

### Phase 3: Entity Management
**Goal**: Full CRUD operations for entities

- [ ] **Entity Creation**: Spawn new entities with optional component templates
- [ ] **Entity Deletion**: Remove entities with confirmation dialogs
- [ ] **Entity Cloning**: Duplicate entities with all their components
- [ ] **Bulk Operations**: Multi-select and batch operations
- [ ] **Entity Search**: Filter entities by ID, components, or custom criteria

### Phase 4: Advanced UI/UX
**Goal**: Professional-grade interface matching industry standards

- [ ] **Tabbed Interface**: Separate views for Entities, Systems, Resources, and Settings
- [ ] **Tree Views**: Hierarchical display with expand/collapse functionality
- [ ] **Search System**: Global search across entities, components, and systems
- [ ] **Filtering Engine**: Advanced filtering with multiple criteria
- [ ] **Property Grid**: Traditional property editor layout
- [ ] **Toolbar Actions**: Quick access to common operations
- [ ] **Keyboard Shortcuts**: Power-user keyboard navigation
- [ ] **Themes**: Light/dark theme switching

### Phase 5: System Inspector
**Goal**: Monitor and control Bevy systems

- [ ] **System Listing**: Display all registered systems in execution order
- [ ] **Performance Metrics**: Execution time, frequency, and resource usage
- [ ] **System Control**: Enable/disable systems at runtime
- [ ] **Dependency Graph**: Visualize system dependencies and execution order
- [ ] **Schedule Inspection**: View and modify system schedules
- [ ] **Debugging Tools**: Breakpoints and step-through debugging

### Phase 6: Resource Management
**Goal**: Inspect and modify global resources

- [ ] **Resource Browser**: List all registered resources
- [ ] **Resource Editing**: Modify resource values in real-time
- [ ] **Resource Monitoring**: Track resource changes over time
- [ ] **Custom Inspectors**: Plugin system for resource-specific editors

### Phase 7: Advanced Features
**Goal**: Professional development tools

- [ ] **Data Export/Import**: Save and load entity configurations
- [ ] **Scene Management**: Import/export entire scenes
- [ ] **Bookmarks**: Save frequently accessed entities and views
- [ ] **History Tracking**: Complete change history with replay capability
- [ ] **Plugin Architecture**: Extension system for custom inspectors
- [ ] **Remote Debugging**: Connect to applications on different machines
- [ ] **Performance Profiler**: Built-in performance analysis tools

## Architecture

### Core Components
- **EditorPlugin**: Main plugin coordinating all editor functionality
- **Remote Client**: HTTP client handling communication with `bevy_remote`
- **UI Systems**: Bevy UI-based interface with modern styling
- **Event System**: Observer-based architecture for reactive updates
- **State Management**: Centralized state for entities, selection, and connection status

### Communication Flow
```
Inspector ←→ HTTP/JSON-RPC ←→ bevy_remote ←→ Target Bevy App
```

### Key Technologies
- **HTTP Client**: `ureq` for synchronous HTTP requests
- **Serialization**: `serde` and `serde_json` for data handling
- **UI Framework**: Native Bevy UI with custom styling
- **Protocol**: JSON-RPC 2.0 following `bevy_remote` specifications

## Development

### Building from Source
```bash
git clone <repository>
cd bevy_editor
cargo build
```

### Running Tests
```bash
cargo test
```

### Example Applications
```bash
# Run the inspector
cargo run --example inspector

# Run a test application with bevy_remote
cargo run --example basic_app
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## 📝 API Reference

### Core Types

#### `EditorState`
Central state management for the editor:
```rust
pub struct EditorState {
    pub selected_entity_id: Option<u32>,
    pub entities: Vec<RemoteEntity>,
    pub show_components: bool,
    pub connection_status: ConnectionStatus,
}
```

#### `RemoteEntity`
Representation of entities from the remote server:
```rust
pub struct RemoteEntity {
    pub id: u32,
    pub components: Vec<String>, // Display names
    pub full_component_names: Vec<String>, // API-compatible names
}
```

#### `ComponentField`
Structured component field data:
```rust
pub struct ComponentField {
    pub name: String,
    pub field_type: String,
    pub value: serde_json::Value,
    pub is_expandable: bool,
}
```

### Events

#### `EntitiesFetched`
Triggered when entity data is received from the remote server:
```rust
pub struct EntitiesFetched {
    pub entities: Vec<RemoteEntity>,
}
```

#### `ComponentDataFetched`
Triggered when component data is received:
```rust
pub struct ComponentDataFetched {
    pub entity_id: u32,
    pub component_data: String,
}
```

## Configuration

### Connection Settings
Default connection parameters can be modified:
```rust
impl Default for RemoteConnection {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:15702".to_string(),
            fetch_interval: 1.0, // seconds
        }
    }
}
```

### UI Customization
The interface uses a consistent color scheme that can be modified in the styling sections of each UI component.

## Troubleshooting

### Common Issues

**Inspector shows "Disconnected"**
- Ensure your target Bevy application is running
- Verify `bevy_remote` plugin is added to your app
- Check that the application is listening on port 15702

**Entities not appearing**
- Confirm entities exist in your target application
- Check the console for connection errors
- Verify the `bevy_remote` server is responding

**Component data shows as raw JSON**
- This is expected in early versions
- Phase 1 development will improve component display

### Debug Mode
Enable debug logging for detailed connection information:
```bash
RUST_LOG=bevy_editor=debug cargo run --example inspector
```

## 📄 License

This project is dual-licensed under:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

You may choose either license for your use.

## Acknowledgments

- **Bevy Engine**: The amazing game engine this editor is built for
- **Flecs Explorer**: Inspiration for the interface design and feature set
- **bevy_remote**: The foundation that makes remote inspection possible
- **Community**: All contributors and users helping shape this tool

---

*Built for the Bevy community*
