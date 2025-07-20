# Bevy Editor

A real-time inspector and editor for Bevy applications. Connect to any Bevy app and inspect entities, components, and systems as they run.

![Bevy Editor](https://img.shields.io/badge/status-in_development-yellow.svg)
![Bevy](https://img.shields.io/badge/bevy-0.15-blue.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)

## Features

### What works right now
- **Real-time Connection**: HTTP client that talks to `bevy_remote`
- **Entity Inspection**: Click through entities and see what's attached
- **Component Viewing**: Components show up as expandable trees instead of raw JSON
- **Smart Type Recognition**: Vec3, Quat, Color, and other Bevy types get nice formatting
- **Connection Status**: Shows if you're connected and when things break
- **Dark Theme UI**: Easy on the eyes
- **Event-driven**: Uses Bevy's observer system under the hood
- **Expandable Structures**: Hit E/T/C keys to expand/collapse component fields
- **Mouse Wheel Scrolling**: Scroll through entity lists (still has some bugs)
- **Dynamic Expansion**: [+]/[-] buttons that actually reflect current state

### Coming up
- **Component Editing**: Actually change values, not just look at them
- **Entity Management**: Create, delete, copy entities
- **Search & Filter**: Find stuff without scrolling forever
- **Hierarchical Views**: Group entities by type and other useful ways
- **System Inspector**: See what systems are running and how long they take
- **Data Persistence**: Save interesting configurations

## Quick Start

### What you need
- Rust 1.70+ 
- Bevy 0.15+
- A Bevy app with `bevy_remote` enabled

### Setup

1. **Add to your project**:
```toml
[dependencies]
bevy_editor = { path = "path/to/bevy_editor" }
```

2. **Add bevy_remote to the app you want to inspect**:
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

3. **Start the inspector**:
```bash
cargo run --example inspector --package bevy_editor
```

4. **Run your Bevy app**

The inspector connects to `http://127.0.0.1:15702` automatically.

## Usage

### Basic workflow
1. **Start the inspector** 
2. **Run your Bevy app** with `bevy_remote` enabled
3. **Click entities** in the left panel to see their components
4. **Browse components** in the right panel
5. **Check connection status** in the top bar if things aren't working

### Connection details
Connects to `bevy_remote` servers at:
- **URL**: `http://127.0.0.1:15702`
- **Protocol**: JSON-RPC 2.0 over HTTP
- **Updates**: Every second

### How components look
Instead of raw JSON, you get something readable:
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

### Controls

Right now:

1. **Click entities** to select them
2. **Components show up** in the right panel with a tree view
3. **Keyboard shortcuts**: 
   - `E` to expand common fields (translation, rotation, scale, etc.)
   - `T` to toggle Transform fields
   - `C` to collapse everything
4. **Mouse wheel** to scroll the entity list
5. **Connection status** shows up top if something's wrong

## Development Roadmap

### Widget System
**Goal**: Build reusable widgets that could eventually be extracted to bevy_feathers

- [x] **BasicPanel**: Simple panels with titles
- [x] **ExpansionButton**: Clickable expand/collapse buttons
- [x] **Theme Integration**: Consistent colors and styling
- [x] **Plugin Architecture**: Each widget is its own plugin

**Details**:
- Designed to be extracted to bevy_feathers later
- Minimal dependencies on core Bevy stuff
- Plugin-based so you can pick what you need
- Consistent APIs across widgets
- Theme system so everything looks coherent
- Theme integration for styling consistency


### Phase 1: Better Component Display
**Goal**: Make raw JSON actually readable

- [x] **Structured Parsing**: Turn JSON into proper typed fields
- [x] **Type-aware Display**: Special handling for Vec3, Quat, Color, etc.
- [x] **Expandable Structures**: [+] buttons for nested stuff
- [x] **Value Formatting**: Entity IDs, truncated strings, sensible number precision
- [x] **Hierarchical Layout**: Proper indentation and nesting
- [x] **Interactive Expansion**: E/T/C keyboard shortcuts with state tracking
- [ ] **Mouse Wheel Scrolling**: Scroll through entity lists
- [ ] **Clickable Expansion**: Replace keyboard shortcuts with actual buttons
- [ ] **Visual Polish**: Better spacing and visual hierarchy
- [ ] **More Type Support**: Asset handles, Entity references, etc.

#### What's left in Phase 1

**Interactive Expansion** (mostly done):
- Expansion state tracking works
- Dynamic [+]/[-] indicators
- Smart keyboard shortcuts for common fields
- Works with any component type, not just Transform

**Mouse Wheel Scrolling** (NOT WORKING):
- ScrollableArea component marks what can scroll
- Mouse wheel handler for entity list
- Smooth scrolling at 5px per wheel tick

**Visual Polish** (next up):
- Color coding for different value types
- Better spacing between items
- Visual distinction between expandable and regular items
- Hover effects

**More Type Support**:
- Asset handle formatting ("Handle<Mesh>", etc.)
- Entity reference formatting with clickable links
- Better support for built-in Bevy components
- Custom type registration for user components

### Phase 2: Component Editing
**Goal**: Actually change values, not just look at them

- [ ] **Input Fields**: Sliders, text boxes, checkboxes - whatever makes sense for the type
- [ ] **Real-time Updates**: Changes get sent to the app immediately
- [ ] **Validation**: Catch bad values before they break things
- [ ] **Error Handling**: Deal with server errors gracefully
- [ ] **Undo/Redo**: Basic history so you can back out of mistakes

### Phase 3: Entity Management
**Goal**: Create, delete, and mess with entities

- [ ] **Entity Creation**: Spawn new entities with component templates
- [ ] **Entity Deletion**: Remove entities (with confirmation so you don't delete the wrong thing)
- [ ] **Entity Cloning**: Copy entities with all their components
- [ ] **Bulk Operations**: Select multiple entities and do things to all of them
- [ ] **Entity Search**: Find entities by ID, components, or whatever

### Phase 4: Better UI
**Goal**: Make the interface not suck

- [ ] **Tabbed Interface**: Separate tabs for Entities, Systems, Resources, Settings
- [ ] **Tree Views**: Hierarchical display with expand/collapse
- [ ] **Search System**: Find stuff across entities, components, systems
- [ ] **Filtering**: Filter by multiple criteria
- [ ] **Property Grid**: Traditional property editor layout
- [ ] **Toolbar**: Quick access to common stuff
- [ ] **Keyboard Shortcuts**: Power-user navigation
- [ ] **Themes**: Light/dark theme switching

### Phase 5: System Inspector
**Goal**: See what systems are doing

- [ ] **System List**: Show all systems in execution order
- [ ] **Performance Metrics**: How long systems take, how often they run
- [ ] **System Control**: Turn systems on/off at runtime
- [ ] **Dependency Graph**: Visualize how systems depend on each other
- [ ] **Schedule Inspection**: See and modify system schedules
- [ ] **Debugging**: Breakpoints and step-through debugging

### Phase 6: Resource Management
**Goal**: Inspect and modify global resources

- [ ] **Resource Browser**: List all registered resources
- [ ] **Resource Editing**: Change resource values in real-time
- [ ] **Resource Monitoring**: Track how resources change over time
- [ ] **Custom Inspectors**: Plugin system for resource-specific editors

### Phase 7: Advanced Stuff
**Goal**: Power user features

- [ ] **Data Export/Import**: Save and load entity configurations
- [ ] **Scene Management**: Import/export entire scenes
- [ ] **Bookmarks**: Save frequently accessed entities and views
- [ ] **History Tracking**: Complete change history with replay
- [ ] **Plugin Architecture**: Extension system for custom inspectors
- [ ] **Remote Debugging**: Connect to apps on different machines
- [ ] **Performance Profiler**: Built-in performance analysis

## How it works

### Core parts
- **EditorPlugin**: Main plugin that coordinates everything
- **Remote Client**: HTTP client that talks to `bevy_remote`
- **UI Systems**: Bevy UI-based interface with custom styling
- **Event System**: Uses Bevy's observer system for reactive updates
- **State Management**: Keeps track of entities, selection, and connection status

### Communication
```
Inspector ←→ HTTP/JSON-RPC ←→ bevy_remote ←→ Your Bevy App
```

### Built with
- **HTTP Client**: `ureq` for HTTP requests
- **Serialization**: `serde` and `serde_json` for data handling
- **UI Framework**: Native Bevy UI with custom styling
- **Protocol**: JSON-RPC 2.0 (what `bevy_remote` uses)

## Development

### Building
```bash
git clone <repository>
cd bevy_editor
cargo build
```

### Testing
```bash
cargo test
```

### Examples
```bash
# Run the inspector
cargo run --example inspector

# Run a test app with bevy_remote
cargo run --example basic_app
```

### Contributing
1. Fork the repo
2. Create a feature branch
3. Make your changes
4. Add tests if needed
5. Submit a pull request

## API Reference

### Important types

#### `EditorState`
Keeps track of what's happening in the editor:
```rust
pub struct EditorState {
    pub selected_entity_id: Option<u32>,
    pub entities: Vec<RemoteEntity>,
    pub show_components: bool,
    pub connection_status: ConnectionStatus,
}
```

#### `RemoteEntity`
What we know about entities from the remote server:
```rust
pub struct RemoteEntity {
    pub id: u32,
    pub components: Vec<String>, // Display names
    pub full_component_names: Vec<String>, // API-compatible names
}
```

#### `ComponentField`
Parsed component field data:
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
Fired when we get entity data from the remote server:
```rust
pub struct EntitiesFetched {
    pub entities: Vec<RemoteEntity>,
}
```

#### `ComponentDataFetched`
Fired when we get component data:
```rust
pub struct ComponentDataFetched {
    pub entity_id: u32,
    pub component_data: String,
}
```

## Configuration

### Connection settings
You can change the default connection if needed:
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

### Customizing the UI
The interface uses a consistent color scheme. You can modify the styling in each UI component if you want different colors.

## Troubleshooting

### Common problems

**Inspector shows "Disconnected"**
- Make sure your Bevy app is actually running
- Check that you added the `bevy_remote` plugin to your app
- Make sure it's listening on port 15702

**No entities showing up**
- Check that your app actually has entities
- Look at the console for connection errors
- Make sure the `bevy_remote` server is responding

**Component data looks like raw JSON**
- This is normal in early versions
- Better component display is coming in Phase 1

### Debug mode
Turn on debug logging to see what's happening:
```bash
RUST_LOG=bevy_editor=debug cargo run --example inspector
```

## 📄 License

This project is dual-licensed under:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

You may choose either license for your use.

## Acknowledgments

- **Bevy Engine**: The game engine this is built for
- **Flecs Explorer**: Inspiration for the interface design
- **bevy_remote**: Makes remote inspection possible
- **Community**: Everyone who's contributed and given feedback

---

*Built for the Bevy community*
