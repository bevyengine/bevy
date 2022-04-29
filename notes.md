* Sync Camera logical and physical sizes
* add new clear color extraction
* Decide RenderGraphRun name (is it a RenderTarget?)
* Define default target priority as 0, support negative numbers, and set "non default" target default to 1


## Summary

This adds "high level camera driven rendering" to Bevy. The goal is to give users more control over what gets rendered (and where) without needing to deal with render logic. This will make scenarios like "render to texture", "multiple windows", "split screen", "2d on 3d", "3d on 2d", "pass layering", and more significantly easier. 

For example, users can now spawn a camera, point it at a RenderTarget (a texture or a window), and it will "just work". 

Rendering to a second window is as simple as spawning a second camera and assigning it to a specific window id:
```rust
// main camera (main window)
commands.spawn_bundle(Camera2dBundle::default());

// second camera (other window)
commands.spawn_bundle(Camera2dBundle {
    camera: Camera {
        target: RenderTarget::Window(window_id),
        ..default()
    },
    ..default()
});
```

Rendering to a texture is as simple as pointing the camera at a texture:

```rust
commands.spawn_bundle(Camera2dBundle {
    camera: Camera {
        target: RenderTarget::Texture(image_handle),
        ..default()
    },
    ..default()
});
```

Cameras now have a "render priority", which controls the order they are drawn in. If you want to use a camera's output texture as a texture in the main pass, just set the priority to a number lower than the main pass camera (which defaults to `0`).

```rust
// main pass camera with a default priority of 0
commands.spawn_bundle(Camera2dBundle::default());

commands.spawn_bundle(Camera2dBundle {
    camera: Camera {
        target: RenderTarget::Texture(image_handle.clone()),
        priority: -1,
        ..default()
    },
    ..default()
});

commands.spawn_bundle(SpriteBundle {
    texture: image_handle,
    ..default()
})
```

Priority can also be used to layer to cameras on top of each other for the same RenderTarget. This is what "2d on top of 3d" looks like in the new system:

```rust
commands.spawn_bundle(Camera3dBundle::default());

commands.spawn_bundle(Camera2dBundle {
    camera: Camera {
        // this will render 2d entities "on top" of the default 3d camera's render
        priority: 1,
        ..default()
    },
    ..default()
});
```

There is no longer the concept of a global "active camera". Resources like `ActiveCamera<Camera2d>` and `ActiveCamera<Camera3d>` have been replaced with the camera-specific `Camera::is_active` field. This does put the onus on users to manage which cameras should be active.

Cameras are now assigned a single render graph as an "entry point", which is configured on each camera entity using the new `CameraRenderGraph` component. The old `PerspectiveCameraBundle` and `OrthographicCameraBundle` (generic on camera marker components like Camera2d and Camera3d) have been replaced by the `Camera3dBundle` and `Camera2dBundle`, which set 3d and 2d default values for the `CameraRenderGraph` and projections.

```rust
// old 3d perspective camera
commands.spawn_bundle(PerspectiveCameraBundle::default())

// new 3d perspective camera
commands.spawn_bundle(Camera3dBundle::default())
```

```rust
// old 2d orthographic camera
commands.spawn_bundle(OrthographicCameraBundle::new_2d())

// new 2d orthographic camera
commands.spawn_bundle(Camera2dBundle::default())
```

```rust
// old 3d orthographic camera
commands.spawn_bundle(OrthographicCameraBundle::new_3d())

// new 3d orthographic camera
commands.spawn_bundle(Camera3dBundle {
    projection: OrthographicProjection {
        scale: 3.0,
        scaling_mode: ScalingMode::FixedVertical,
        ..default()
    }.into(),
    ..default()
})
```

Note that `Camera3dBundle` now uses a new `Projection` enum instead of hard coding the projection into the type. There are a number of motivators for this change: the render graph is now a part of the bundle, the way "generic bundles" work in the rust type system prevents nice `..default()` syntax, and changing projections at runtime is much easier with an enum (ex for editor scenarios). I'm open to discussing this choice, but I'm relatively certain we will all come to the same conclusion here. Camera2dBundle and Camera3dBundle are much clearer than being generic on marker components / using non-default constructors.

If you want to run a custom render graph on a camera, just set the `CameraRenderGraph` component:

```rust
commands.spawn_bundle(Camera3dBundle {
    camera_render_graph: CameraRenderGraph::new(some_render_graph_name),
    ..default()
})
```

Just note that if the graph requires data from specific components to work (such as `Camera3d` config, which is provided in the `Camera3dBundle`), make sure the relevant components have been added.

Speaking of using components to configure graphs / passes, there are a number of new configuration options:

```rust
commands.spawn_bundle(Camera3dBundle {
    camera_3d: Camera3d {
        // overrides the default global clear color 
        clear_color: ClearColorConfig::Custom(Color::RED),
        ..default()
    },
    ..default()
})

commands.spawn_bundle(Camera3dBundle {
    camera_3d: Camera3d {
        // disables clearing
        clear_color: ClearColorConfig::None,
        ..default()
    },
    ..default()
})
```

Expect to see more of the "graph configuration Components on Cameras" pattern in the future.

By popular demand, UI no longer requires a dedicated camera. `UiCameraBundle` has been removed. `Camera2dBundle` and `Camera3dBundle` now both default to rendering UI as part of their own render graphs. To disable UI rendering for a camera, disable it using the CameraUi component:

```rust
commands
    .spawn_bundle(Camera3dBundle::default())
    .insert(CameraUi {
        is_enabled: false,
        ..default()
    })
```

## Other Changes

* The separate clear pass has been removed. We should revisit this for things like sky rendering, but I think this PR should "keep it simple" until we're ready to properly support that (for code complexity and performance reasons). We can come up with the right design for a modular clear pass in a followup pr.
* I reorganized bevy_core_pipeline into Core2dPlugin and Core3dPlugin (and core_2d / core_3d modules). Everything is pretty much the same as before, just logically separate. I've moved relevant types (like Camera2d, Camera3d, Camera3dBundle, Camera2dBundle) into their relevant modules, which is what motivated this reorganization.
* I adapted the `scene_viewer` example (which relied on the ActiveCameras behavior) to the new system. I also refactored bits and pieces to be a bit simpler. 
* All of the examples have been ported to the new camera approach. `render_to_texture` and `multiple_windows` are now _much_ simpler. I removed `two_passes` because it is less relevant with the new approach. If someone wants to add a new `layered custom pass with CameraRenderGraph` example, that might fill a similar niche. But I don't feel much pressure to add that in this pr.
* Render order ambiguities between cameras with the same target and the same priority now produce a warning. This accomplishes two goals:
    1. Now that there is no "global" active camera, by default spawning two cameras will result in two renders (one covering the other). This would be a silent performance killer that would be hard to detect after the fact. By detecting ambiguities, we can provide a helpful warning when this occurs.
    2. Render order ambiguities could result in unexpected / unpredictable render results. Resolving them makes sense.

## Follow Up Work

* Per-Camera viewports, which will make it possible to render to a smaller area inside of a RenderTarget (great for something like splitscreen)
* Camera-specific MSAA config (should use the same "overriding" pattern used for ClearColor)
* Graph Based Camera Ordering: priorities are simple, but they make complicated ordering constraints harder to express. We should consider adopting a "graph based" camera ordering model with "before" and "after" relationships to other cameras (or build it "on top" of the priority system).
* Consider allowing graphs to run subgraphs from any nest level (aka a global namespace for graphs). Right now the 2d and 3d graphs each need their own UI subgraph, which feels "fine" in the short term. But being able to share subgraphs between other subgraphs seems valuable.
* Consider splitting `bevy_core_pipeline` into `bevy_core_2d` and `bevy_core_3d` packages. Theres a shared "clear color" dependency here, which would need a new home.

## TODO

* Consolidate update_frusta

## Scratch RenderGraphRun driven

* RenderGraphRun component drives graphs
    * Has RenderTarget
    * Has ActiveCamera<Camera3d> and ActiveCamera<Camera2d> 
* Cameras point to RenderGraphRun entities
* How to get the default RenderGraphRun: stored in an entity?
    * DefaultRenderGraphRun(Entity)?

```rust
fn system(query: Query<&RenderGraphRun>, default: Res<DefaultRenderGraphRun>) {

}
```

## Scratch Camera Driven


* Camera entities drive graphs
    * Has RenderTarget
    * Active is a boolean toggle
    * Clear Color is defined on the Camera 
* Cameras have projections, can generate more than one using components on camera
    * probably best to just remove manual UI projections

* 2d on 3d
    * 3d first, disable UI
    * 2d next, enable UI, disable clear color
