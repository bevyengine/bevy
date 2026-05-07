---
title: "Infinite Grid"
authors: [ "@IceSentry" ]
pull_requests: [ 23482 ]
---

*TODO: Add a screenshot of the infinite grid with a real model.*

A transparent ground-plane grid is a staple of 3D editor tooling: it marks the major axes, orients the scene, and makes scale immediately legible.

Simply drawing lines doesn't work well: the mesh has to end somewhere, and the lines that reach toward the horizon create aliasing artifacts and Moiré patterns no matter how far you extend it.

Our implementation renders the grid as a fullscreen shader: the grid is computed per-pixel in screen space from the camera's perspective, and fades out with distance to eliminate aliasing at the horizon.

To add an infinite grid to your app, register `InfiniteGridPlugin` and spawn the `InfiniteGrid` component:

```rust
use bevy::dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin};
use bevy::prelude::*;

App::new()
    .add_plugins((DefaultPlugins, InfiniteGridPlugin))
    .add_systems(Startup, setup)
    .run();

fn setup(mut commands: Commands) {
    commands.spawn(InfiniteGrid);
}
```

Grid appearance — colors, fade distance, line scale — is controlled by `InfiniteGridSettings`, which can be placed on the grid entity or on a specific camera to override it per-view:

```rust
use bevy::dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridSettings};

// On the grid entity (applies to all cameras)
commands.spawn((
    InfiniteGrid,
    InfiniteGridSettings {
        fadeout_distance: 200.0,
        ..default()
    },
));

// On a camera (overrides settings for that camera only)
commands.spawn((
    Camera3d::default(),
    InfiniteGridSettings {
        scale: 0.5,
        ..default()
    },
));
```

This is an upstreamed version of the [`bevy_infinite_grid` crate], created and maintained by Foresight Spatial Labs — thank you for building it and generously contributing it to Bevy!

[`bevy_infinite_grid` crate]: https://github.com/fslabs/bevy_infinite_grid
