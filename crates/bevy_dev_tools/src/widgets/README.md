# Bevy UI Widgets

This module contains reusable UI widgets designed for use with the Bevy game engine. These widgets were extracted from the bevy_dev_tools inspector and are suitable for upstreaming to bevy_ui.

## Features

Each widget is designed to be:

- **Modular**: Self-contained with minimal dependencies
- **Performant**: Optimized for real-world usage  
- **Extensible**: Easy to customize and extend
- **Well-documented**: Clear API and usage examples

## Available Widgets

### 1. SelectableText

Text with selection and clipboard copy functionality.

**Features:**

- Click to select text
- Ctrl+C to copy to clipboard
- Escape to clear selection
- Visual selection feedback
- Cross-platform clipboard support

**Usage:**

```rust
use bevy::prelude::*;
use bevy_dev_tools::widgets::{SelectableText, SelectableTextPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SelectableTextPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    commands.spawn((
        Button,
        Text::new("Click me to select, then Ctrl+C to copy"),
        SelectableText::new("Click me to select, then Ctrl+C to copy"),
        // ... other UI components
    ));
}
```

**API:**

- `SelectableText::new(content)`: Create selectable text component
- `select_all()`: Select all text programmatically
- `clear_selection()`: Clear current selection
- `selected_text()`: Get currently selected text

### 2. CollapsibleSection

Expandable/collapsible content sections.

**Features:**

- Clickable headers to expand/collapse
- Customizable styling
- Nested sections support
- Smooth expand/collapse animations
- Custom arrow indicators

**Usage:**

```rust
use bevy::prelude::*;
use bevy_dev_tools::widgets::{CollapsibleSectionPlugin, spawn_collapsible_section};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CollapsibleSectionPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    let root = commands.spawn((
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
    )).id();
    
    // Create collapsible section
    let section = spawn_collapsible_section(
        &mut commands,
        root,
        "My Collapsible Section"
    );
    
    // Add content to the section
    commands.entity(section).with_children(|section| {
        // Content goes here...
        section.spawn((
            Text::new("This content can be collapsed!"),
            // ... other components
        ));
    });
}
```

**API:**

- `spawn_collapsible_section(commands, parent, title)`: Create basic section
- `spawn_collapsible_section_with_config(commands, parent, config)`: Create with custom config
- `CollapsibleSection::new(title)`: Builder pattern configuration
- `CollapsibleStyle`: Customize colors, fonts, and spacing

### 3. VirtualScrolling

High-performance scrolling for large lists.

**Features:**

- Virtual rendering (only visible items + buffer)
- Smooth momentum scrolling
- Adaptive buffering during fast scrolling
- Optional scrollbar indicator
- Generic over any content type
- Frame-rate limiting for performance

**Usage:**

```rust
use bevy::prelude::*;
use bevy_dev_tools::widgets::{
    VirtualScrollPlugin, VirtualScrollable, spawn_virtual_scroll_container,
    handle_virtual_scroll_input, update_virtual_scroll_display
};

// Define your scrollable item type
#[derive(Component, Clone)]
struct MyItem {
    text: String,
}

impl VirtualScrollable for MyItem {
    fn spawn_ui(&self, commands: &mut Commands, parent: Entity, index: usize, item_height: f32) {
        commands.entity(parent).with_children(|parent| {
            parent.spawn((
                Text::new(&self.text),
                // ... styling components
            ));
        });
    }
    
    fn get_id(&self) -> u64 {
        // Return unique ID for this item
        self.text.len() as u64
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VirtualScrollPlugin)
        .add_systems(Startup, (setup, setup_virtual_scrolling::<MyItem>))
        .add_systems(Update, (
            handle_virtual_scroll_input::<MyItem>,
            update_virtual_scroll_display::<MyItem>,
            update_scroll_momentum::<MyItem>,
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    let root = commands.spawn((
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            ..Default::default()
        },
    )).id();
    
    // Create virtual scroll container
    let (_container, _content) = spawn_virtual_scroll_container::<MyItem>(
        &mut commands,
        root,
        Val::Percent(100.0), // width
        Val::Percent(100.0), // height
        true, // with_scrollbar
    );
}
```

**API:**

- `VirtualScrollable` trait: Implement for your item type
- `spawn_virtual_scroll_container()`: Create scrollable container
- `VirtualScrollState<T>`: Resource managing scroll state
- Various systems for input handling and display updates

## Cross-Platform Support

All widgets are tested on Windows, macOS, and Linux. The clipboard functionality gracefully degrades on unsupported platforms.

## Performance

- **SelectableText**: Minimal overhead, only processes events when needed
- **CollapsibleSection**: Efficient show/hide using CSS display properties
- **VirtualScrolling**: Maintains ~50-100 UI elements regardless of total item count

## Integration with Inspector

These widgets were originally developed for the bevy_dev_tools inspector and can be seen in action there:

```bash
cargo run --example inspector_minimal --features="bevy_dev_tools"
```

## Future Improvements

Potential enhancements for upstreaming to bevy_ui:

1. **SelectableText**:
   - Multi-line text selection
   - Text editing capabilities
   - Better cursor positioning

2. **CollapsibleSection**:
   - Animation system integration
   - Accessibility improvements
   - Keyboard navigation

3. **VirtualScrolling**:
   - Horizontal scrolling support
   - Grid layouts
   - More sophisticated buffering strategies

## Contributing

When modifying these widgets:

1. Maintain backward compatibility
2. Add comprehensive tests
3. Update documentation
4. Consider performance implications
5. Test on all supported platforms

## License

These widgets are part of bevy_dev_tools and follow the same license as Bevy (MIT OR Apache-2.0).
