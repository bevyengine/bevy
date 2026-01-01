---
title: Automatic Directional Navigation
authors: ["@jbuehler23"]
pull_requests: [21668]
---

Bevy now supports **automatic directional navigation graph generation** for UI elements! No more tedious manual wiring of navigation connections for your menus and UI screens.

## What's New?

Previously, creating directional navigation for UI required manually defining every connection between focusable elements using `DirectionalNavigationMap`. For dynamic UIs or complex layouts, this was time-consuming and error-prone.

Now, you can simply add the `AutoDirectionalNavigation` component to your UI entities, and Bevy will automatically compute navigation connections based on spatial positioning. The system intelligently finds the nearest neighbor in each of the 8 compass directions (North, Northeast, East, etc.), considering:

- **Distance**: Closer elements are preferred
- **Alignment**: Elements that are more directly in line with the navigation direction are favored
- **Overlap**: For cardinal directions (N/S/E/W), the system ensures sufficient perpendicular overlap

## How to Use It

Simply add the `AutoDirectionalNavigation` component to your UI entities:

```rust
commands.spawn((
    Button,
    Node { /* ... */ },
    AutoDirectionalNavigation::default(),
    // ... other components
));
```

And use the new `AutoDirectionalNavigator` system parameter instead of `DirectionalNavigation`.

That's it! The navigator will consider any entities with the `AutoDirectionalNavigation` component when navigating.

### Configuration

You can tune the behavior using the `NavigatorConfig` resource:

```rust
app.insert_resource(NavigatorConfig {
    // Minimum overlap required (0.0 = any overlap, 1.0 = perfect alignment)
    min_alignment_factor: 0.0,
    // Optional maximum distance for connections
    max_search_distance: Some(500.0),
    // Whether to strongly prefer well-aligned nodes
    prefer_aligned: true,
});
```

### Manual Override

Automatic navigation respects manually-defined edges. If you want to override specific connections, you can still use `DirectionalNavigationMap::add_edge()` or `add_symmetrical_edge()`, and those connections will take precedence over the auto-generated ones.
You may also call `auto_generate_navigation_edges()` directly, if you have multiple UI layers (though may not be widely used)

## Why This Matters

This feature dramatically simplifies UI navigation setup:

- **Less boilerplate**: No need to manually wire up dozens or hundreds of navigation connections
- **Works with dynamic UIs**: Automatically adapts when UI elements are added, removed, or repositioned
- **Flexible**: Mix automatic and manual navigation as needed
- **Configurable**: Tune the algorithm to match your UI's needs

Whether you're building menus, inventory screens, or any other gamepad/keyboard-navigable UI, automatic directional navigation makes it much easier to create intuitive, responsive navigation experiences.

## Migration Guide

This is a non-breaking change. Existing manual navigation setups continue to work as before.

If you want to convert existing manual navigation to automatic:

**Before:**

```rust
// Manually define all edges
directional_nav_map.add_looping_edges(&row_entities, CompassOctant::East);
directional_nav_map.add_edges(&column_entities, CompassOctant::South);
// ... repeat for all rows and columns
```

```rust
// Use the DirectionalNavigation SystemParam to navigate in your system
fn navigation_system(mut directional_navigation: DirectionalNavigation) {
    // ...
    directional_navigation.navigate(CompassOctant::East);
    // ...
```

**After:**

```rust
// Just add the component to your UI entities
commands.spawn((
    Button,
    Node { /* ... */ },
    AutoDirectionalNavigation::default(),
));
```

```rust
// Use the AutoDirectionalNavigator SystemParam to navigate
fn navigation_system(mut auto_directional_navigator: AutoDirectionalNavigator) {
    // ...
    auto_directional_navigator.navigate(CompassOctant::East);
    // ...
```

Note: The automatic navigation system requires entities to have position and size information (`ComputedNode` and `UiGlobalTransform` for `bevy_ui` entities).
