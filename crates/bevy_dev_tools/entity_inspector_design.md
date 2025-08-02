
# Bevy Entity/Component Inspector - Design Document

- **Status:** Proposed
- **Owner:** TBD

## 1. Goal

To design and build a comprehensive, standalone entity/component inspector as a debug tool for Bevy applications. The inspector must be implemented using only Bevy-native UI libraries, operate in its own window, and be extensible to support various data sources.

This tool will allow developers to pause and inspect the state of their application's `World` at runtime, providing a much-needed "in-engine" debugging experience without relying on external tools like `egui`.

## 2. Core Principles

- **Bevy Native:** The UI and all logic will be built on core Bevy APIs. It will exclusively use widgets and rendering provided by `bevy_ui` and other blessed ecosystem crates like `bevy_feathers` or `bevy_core_widgets`. No external UI frameworks.
- **No `bevy::prelude`:** To ensure the inspector is a good citizen within the Bevy ecosystem, it will not use the global `bevy::prelude`. Instead, all Bevy types will be imported directly from their specific crates (e.g., `bevy_app`, `bevy_ecs`, `bevy_ui`).
- **Standalone Plugin:** The inspector will be delivered as a single, easy-to-add plugin (`EntityInspectorPlugin`). A developer should be able to add it to their app with a single line: `app.add_plugins(EntityInspectorPlugin);`.
- **Extensible Data Sources:** The inspector's architecture must be flexible enough to display data from different sources, not just the local `World`. The initial design will account for:
    - **Local Data:** The live `World` of the running application.
    - **Remote Data:** Data streamed from another Bevy application via `bevy_remote`.
    - **Scene Files:** Data loaded from a `.scn.ron` file.
- **Performance Conscious:** The inspector should have a minimal performance impact when closed and a reasonable impact when open. UI updates should be efficient.

## 3. Proposed Architecture

### 3.1. `EntityInspectorPlugin`

The public-facing API. This plugin is responsible for:
- Adding all necessary systems, resources, and states.
- Setting up a toggle mechanism (e.g., listening for a specific key press like `F12`) to open and close the inspector window.
- Managing the `InspectorState` (`Active` vs. `Inactive`).

### 3.2. Window Management

- On activation, the plugin will spawn a new `Window` entity dedicated to the inspector.
- All UI elements for the inspector will be rendered to this secondary window's render target.
- A separate `Camera` entity will be configured for the inspector's UI.
- When the inspector is closed, this `Window` entity and all associated UI entities will be despawned.

### 3.3. UI Layer

The UI will be constructed with `bevy_ui` nodes and styled. It will consist of a two-pane layout:

- **Left Pane (Entity List):**
    - A scrollable list of all entities from the current data source.
    - Each entity will be a clickable button showing its `Entity` ID and optionally its `Name`.
    - A search/filter bar at the top to quickly find entities.

- **Right Pane (Component View):**
    - Displays the components of the entity selected in the left pane.
    - The view will be dynamically generated using reflection.
    - Each component will be displayed in its own collapsible section, showing its type name.
    - The fields of each component will be recursively displayed using `bevy_reflect` to traverse the data structure.

### 3.4. Data Abstraction Layer

To support multiple data sources, we will define a central resource or trait.

```rust
// Example abstraction
enum InspectorDataSource {
    Local,
    Remote { connection: ConnectionInfo },
    Scene { path: PathBuf },
}

// A system would be responsible for populating a unified data structure
// that the UI can then read from, regardless of the source.
struct InspectedData {
    entities: Vec<(Entity, Option<Name>)>,
    components: HashMap<Entity, Vec<Box<dyn Reflect>>>,
}
```

### 3.5. Reflection-Powered UI Generation

This is the core of the component viewer.
1. When an entity is selected, the system will access the `AppTypeRegistry`.
2. For the given `Entity`, it will iterate through all of its components.
3. For each component that is registered and implements `Reflect`, it will get a `&dyn Reflect` handle.
4. A recursive UI generation function will take this `&dyn Reflect` handle and build a tree of `bevy_ui` nodes to represent it.
    - `Struct`: Create a nested section with labels for each field.
    - `List`: Create a list of its elements.
    - `Value`: Display the value using `format!("{:?}", ...)` in a `Text` component.

## 4. Component & Implementation Roadmap

### Milestone 1: Core Plugin and Window
- Create the `EntityInspectorPlugin`.
- Add a system to listen for a key press to toggle an `InspectorState`.
- On entering the `Active` state, spawn a new `Window` with a title like "Bevy Inspector".
- On exiting, despawn the window.

### Milestone 2: Basic UI and Local Data
- Implement the basic two-pane UI layout using `bevy_ui` nodes.
- Create a system that queries for all `Entity`s in the local `World`.
- Populate the left pane with a scrollable list of these entities.
- Store the currently selected entity in a resource.

### Milestone 3: Reflection-Based Component Viewer (Read-Only)
- Create a system that runs when the selected entity changes.
- This system will use `bevy_reflect` to get the components of the selected entity.
- Dynamically generate and spawn `Text` nodes in the right pane to display the component data.
- Implement collapsible sections for each component.

### Milestone 4: UI Polish and Interactivity
- Add a `TextInput` widget for searching/filtering the entity list.
- Improve styling and layout to be more compact and readable.
- Ensure scroll views are robust.

### Milestone 5: Advanced Data Sources
- Refactor the data-gathering systems to work with the `InspectorDataSource` abstraction.
- Implement the `SceneFileDataSource` which can be triggered via a "Load Scene" button.
- Implement the `RemoteDataSource` to connect to a `bevy_remote` stream.

### Milestone 6 (Future): Component Value Editing
- Extend the reflection-based UI generator to create interactive widgets for mutable fields.
    - `bool`: Checkbox
    - `f32`, `u32`, etc.: Draggable slider or text input.
    - `Vec3`: Three numeric inputs.
    - `String`: Text input.
- Use `ReflectMut` to apply the changes back to the component data.

## 5. Open Questions

- How performant will a complex, deeply nested UI be using only `bevy_ui`?
- What is the best approach for creating generic, reusable "editor" widgets (e.g., a `Vec3` editor) for reflected types?
- What is the final API for `bevy_remote` and how will we integrate with its data stream?
