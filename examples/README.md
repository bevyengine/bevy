<!-- MD024 - The Headers from the Platform-Specific Examples should be identical  -->
<!-- Use 'cargo run -p build-templated-pages -- build-example-page' to generate the final example README.md -->
<!-- markdownlint-disable-file MD024 -->

# Examples

These examples demonstrate the main features of Bevy and how to use them.
To run an example, use the command `cargo run --example <Example>`, and add the option `--features x11` or `--features wayland` to force the example to run on a specific window compositor, e.g.

```sh
cargo run --features wayland --example hello_world
```

**⚠️ Note: for users of releases on crates.io!**

There are often large differences and incompatible API changes between the latest [crates.io](https://crates.io/crates/bevy) release and the development version of Bevy in the git main branch!

If you are using a released version of bevy, you need to make sure you are viewing the correct version of the examples!

- Latest release: [https://github.com/bevyengine/bevy/tree/latest/examples](https://github.com/bevyengine/bevy/tree/latest/examples)
- Specific version, such as `0.4`: [https://github.com/bevyengine/bevy/tree/v0.4.0/examples](https://github.com/bevyengine/bevy/tree/v0.4.0/examples)

When you clone the repo locally to run the examples, use `git checkout` to get the correct version:

```bash
# `latest` always points to the newest release
git checkout latest
# or use a specific version
git checkout v0.4.0
```

---

## Table of Contents

- [Examples](#examples)
  - [Table of Contents](#table-of-contents)
- [The Bare Minimum](#the-bare-minimum)
  - [Hello, World!](#hello-world)
- [Cross-Platform Examples](#cross-platform-examples)
  - [2D Rendering](#2d-rendering)
  - [3D Rendering](#3d-rendering)
  - [Animation](#animation)
  - [Application](#application)
  - [Assets](#assets)
  - [Async Tasks](#async-tasks)
  - [Audio](#audio)
  - [Camera](#camera)
  - [Dev tools](#dev-tools)
  - [Diagnostics](#diagnostics)
  - [ECS (Entity Component System)](#ecs-entity-component-system)
  - [Games](#games)
  - [Gizmos](#gizmos)
  - [Input](#input)
  - [Math](#math)
  - [Reflection](#reflection)
  - [Scene](#scene)
  - [Shaders](#shaders)
  - [State](#state)
  - [Stress Tests](#stress-tests)
  - [Time](#time)
  - [Tools](#tools)
  - [Transforms](#transforms)
  - [UI (User Interface)](#ui-user-interface)
  - [Window](#window)

- [Tests](#tests)
- [Platform-Specific Examples](#platform-specific-examples)
  - [Android](#android)
    - [Setup](#setup)
    - [Build & Run](#build--run)
    - [Old phones](#old-phones)
  - [iOS](#ios)
    - [Setup](#setup-1)
    - [Build & Run](#build--run-1)
  - [WASM](#wasm)
    - [Setup](#setup-2)
    - [Build & Run](#build--run-2)
    - [WebGL2 and WebGPU](#webgl2-and-webgpu)
    - [Audio in the browsers](#audio-in-the-browsers)
    - [Optimizing](#optimizing)
    - [Loading Assets](#loading-assets)

# The Bare Minimum

<!-- MD026 - Hello, World! looks better with the ! -->
<!-- markdownlint-disable-next-line MD026 -->
## Hello, World!

Example | Description
--- | ---
[`hello_world.rs`](./hello_world.rs) | Runs a minimal example that outputs "hello world"

# Cross-Platform Examples

## 2D Rendering

Example | Description
--- | ---
[2D Bloom](../examples/2d/bloom_2d.rs) | Illustrates bloom post-processing in 2d
[2D Bounding Volume Intersections](../examples/2d/bounding_2d.rs) | Showcases bounding volumes and intersection tests
[2D Rotation](../examples/2d/rotation.rs) | Demonstrates rotating entities in 2D with quaternions
[2D Shapes](../examples/2d/2d_shapes.rs) | Renders simple 2D primitive shapes like circles and polygons
[2D Viewport To World](../examples/2d/2d_viewport_to_world.rs) | Demonstrates how to use the `Camera::viewport_to_world_2d` method
[2D Wireframe](../examples/2d/wireframe_2d.rs) | Showcases wireframes for 2d meshes
[Arc 2D Meshes](../examples/2d/mesh2d_arcs.rs) | Demonstrates UV-mapping of the circular segment and sector primitives
[Custom glTF vertex attribute 2D](../examples/2d/custom_gltf_vertex_attribute.rs) | Renders a glTF mesh in 2D with a custom vertex attribute
[Manual Mesh 2D](../examples/2d/mesh2d_manual.rs) | Renders a custom mesh "manually" with "mid-level" renderer apis
[Mesh 2D](../examples/2d/mesh2d.rs) | Renders a 2d mesh
[Mesh 2D With Vertex Colors](../examples/2d/mesh2d_vertex_color_texture.rs) | Renders a 2d mesh with vertex color attributes
[Move Sprite](../examples/2d/move_sprite.rs) | Changes the transform of a sprite
[Pixel Grid Snapping](../examples/2d/pixel_grid_snap.rs) | Shows how to create graphics that snap to the pixel grid by rendering to a texture in 2D
[Sprite](../examples/2d/sprite.rs) | Renders a sprite
[Sprite Animation](../examples/2d/sprite_animation.rs) | Animates a sprite in response to an event
[Sprite Flipping](../examples/2d/sprite_flipping.rs) | Renders a sprite flipped along an axis
[Sprite Sheet](../examples/2d/sprite_sheet.rs) | Renders an animated sprite
[Sprite Slice](../examples/2d/sprite_slice.rs) | Showcases slicing sprites into sections that can be scaled independently via the 9-patch technique
[Sprite Tile](../examples/2d/sprite_tile.rs) | Renders a sprite tiled in a grid
[Text 2D](../examples/2d/text2d.rs) | Generates text in 2D
[Texture Atlas](../examples/2d/texture_atlas.rs) | Generates a texture atlas (sprite sheet) from individual sprites
[Transparency in 2D](../examples/2d/transparency_2d.rs) | Demonstrates transparency in 2d

## 3D Rendering

Example | Description
--- | ---
[3D Bloom](../examples/3d/bloom_3d.rs) | Illustrates bloom configuration using HDR and emissive materials
[3D Scene](../examples/3d/3d_scene.rs) | Simple 3D scene with basic shapes and lighting
[3D Shapes](../examples/3d/3d_shapes.rs) | A scene showcasing the built-in 3D shapes
[3D Viewport To World](../examples/3d/3d_viewport_to_world.rs) | Demonstrates how to use the `Camera::viewport_to_world` method
[Animated Material](../examples/3d/animated_material.rs) | Shows how to animate material properties
[Anisotropy](../examples/3d/anisotropy.rs) | Displays an example model with anisotropy
[Anti-aliasing](../examples/3d/anti_aliasing.rs) | Compares different anti-aliasing methods
[Atmospheric Fog](../examples/3d/atmospheric_fog.rs) | A scene showcasing the atmospheric fog effect
[Auto Exposure](../examples/3d/auto_exposure.rs) | A scene showcasing auto exposure
[Blend Modes](../examples/3d/blend_modes.rs) | Showcases different blend modes
[Clearcoat](../examples/3d/clearcoat.rs) | Demonstrates the clearcoat PBR feature
[Color grading](../examples/3d/color_grading.rs) | Demonstrates color grading
[Deferred Rendering](../examples/3d/deferred_rendering.rs) | Renders meshes with both forward and deferred pipelines
[Depth of field](../examples/3d/depth_of_field.rs) | Demonstrates depth of field
[Fog](../examples/3d/fog.rs) | A scene showcasing the distance fog effect
[Generate Custom Mesh](../examples/3d/generate_custom_mesh.rs) | Simple showcase of how to generate a custom mesh with a custom texture
[Irradiance Volumes](../examples/3d/irradiance_volumes.rs) | Demonstrates irradiance volumes
[Lighting](../examples/3d/lighting.rs) | Illustrates various lighting options in a simple scene
[Lightmaps](../examples/3d/lightmaps.rs) | Rendering a scene with baked lightmaps
[Lines](../examples/3d/lines.rs) | Create a custom material to draw 3d lines
[Load glTF](../examples/3d/load_gltf.rs) | Loads and renders a glTF file as a scene
[Load glTF extras](../examples/3d/load_gltf_extras.rs) | Loads and renders a glTF file as a scene, including the gltf extras
[Meshlet](../examples/3d/meshlet.rs) | Meshlet rendering for dense high-poly scenes (experimental)
[Motion Blur](../examples/3d/motion_blur.rs) | Demonstrates per-pixel motion blur
[Orthographic View](../examples/3d/orthographic.rs) | Shows how to create a 3D orthographic view (for isometric-look in games or CAD applications)
[Parallax Mapping](../examples/3d/parallax_mapping.rs) | Demonstrates use of a normal map and depth map for parallax mapping
[Parenting](../examples/3d/parenting.rs) | Demonstrates parent->child relationships and relative transformations
[Physically Based Rendering](../examples/3d/pbr.rs) | Demonstrates use of Physically Based Rendering (PBR) properties
[Reflection Probes](../examples/3d/reflection_probes.rs) | Demonstrates reflection probes
[Render to Texture](../examples/3d/render_to_texture.rs) | Shows how to render to a texture, useful for mirrors, UI, or exporting images
[Screen Space Ambient Occlusion](../examples/3d/ssao.rs) | A scene showcasing screen space ambient occlusion
[Screen Space Reflections](../examples/3d/ssr.rs) | Demonstrates screen space reflections with water ripples
[Shadow Biases](../examples/3d/shadow_biases.rs) | Demonstrates how shadow biases affect shadows in a 3d scene
[Shadow Caster and Receiver](../examples/3d/shadow_caster_receiver.rs) | Demonstrates how to prevent meshes from casting/receiving shadows in a 3d scene
[Skybox](../examples/3d/skybox.rs) | Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats.
[Spherical Area Lights](../examples/3d/spherical_area_lights.rs) | Demonstrates how point light radius values affect light behavior
[Split Screen](../examples/3d/split_screen.rs) | Demonstrates how to render two cameras to the same window to accomplish "split screen"
[Spotlight](../examples/3d/spotlight.rs) | Illustrates spot lights
[Texture](../examples/3d/texture.rs) | Shows configuration of texture materials
[Tonemapping](../examples/3d/tonemapping.rs) | Compares tonemapping options
[Transmission](../examples/3d/transmission.rs) | Showcases light transmission in the PBR material
[Transparency in 3D](../examples/3d/transparency_3d.rs) | Demonstrates transparency in 3d
[Two Passes](../examples/3d/two_passes.rs) | Renders two 3d passes to the same window from different perspectives
[Update glTF Scene](../examples/3d/update_gltf_scene.rs) | Update a scene from a glTF file, either by spawning the scene as a child of another entity, or by accessing the entities of the scene
[Vertex Colors](../examples/3d/vertex_colors.rs) | Shows the use of vertex colors
[Visibility range](../examples/3d/visibility_range.rs) | Demonstrates visibility ranges
[Volumetric fog](../examples/3d/volumetric_fog.rs) | Demonstrates volumetric fog and lighting
[Wireframe](../examples/3d/wireframe.rs) | Showcases wireframe rendering

## Animation

Example | Description
--- | ---
[Animated Fox](../examples/animation/animated_fox.rs) | Plays an animation from a skinned glTF
[Animated Transform](../examples/animation/animated_transform.rs) | Create and play an animation defined by code that operates on the `Transform` component
[Animation Graph](../examples/animation/animation_graph.rs) | Blends multiple animations together with a graph
[Color animation](../examples/animation/color_animation.rs) | Demonstrates how to animate colors using mixing and splines in different color spaces
[Cubic Curve](../examples/animation/cubic_curve.rs) | Bezier curve example showing a cube following a cubic curve
[Custom Skinned Mesh](../examples/animation/custom_skinned_mesh.rs) | Skinned mesh example with mesh and joints data defined in code
[Morph Targets](../examples/animation/morph_targets.rs) | Plays an animation from a glTF file with meshes with morph targets
[glTF Skinned Mesh](../examples/animation/gltf_skinned_mesh.rs) | Skinned mesh example with mesh and joints data loaded from a glTF file

## Application

Example | Description
--- | ---
[Advanced log layers](../examples/app/log_layers_ecs.rs) | Illustrate how to transfer data between log layers and Bevy's ECS
[Custom Loop](../examples/app/custom_loop.rs) | Demonstrates how to create a custom runner (to update an app manually)
[Drag and Drop](../examples/app/drag_and_drop.rs) | An example that shows how to handle drag and drop in an app
[Empty](../examples/app/empty.rs) | An empty application (does nothing)
[Empty with Defaults](../examples/app/empty_defaults.rs) | An empty application with default plugins
[Headless](../examples/app/headless.rs) | An application that runs without default plugins
[Headless Renderer](../examples/app/headless_renderer.rs) | An application that runs with no window, but renders into image file
[Log layers](../examples/app/log_layers.rs) | Illustrate how to add custom log layers
[Logs](../examples/app/logs.rs) | Illustrate how to use generate log output
[No Renderer](../examples/app/no_renderer.rs) | An application that runs with default plugins and displays an empty window, but without an actual renderer
[Plugin](../examples/app/plugin.rs) | Demonstrates the creation and registration of a custom plugin
[Plugin Group](../examples/app/plugin_group.rs) | Demonstrates the creation and registration of a custom plugin group
[Return after Run](../examples/app/return_after_run.rs) | Show how to return to main after the Bevy app has exited
[Thread Pool Resources](../examples/app/thread_pool_resources.rs) | Creates and customizes the internal thread pool
[Without Winit](../examples/app/without_winit.rs) | Create an application without winit (runs single time, no event loop)

## Assets

Example | Description
--- | ---
[Asset Decompression](../examples/asset/asset_decompression.rs) | Demonstrates loading a compressed asset
[Asset Loading](../examples/asset/asset_loading.rs) | Demonstrates various methods to load assets
[Asset Processing](../examples/asset/processing/asset_processing.rs) | Demonstrates how to process and load custom assets
[Asset Settings](../examples/asset/asset_settings.rs) | Demonstrates various methods of applying settings when loading an asset
[Custom Asset](../examples/asset/custom_asset.rs) | Implements a custom asset loader
[Custom Asset IO](../examples/asset/custom_asset_reader.rs) | Implements a custom AssetReader
[Embedded Asset](../examples/asset/embedded_asset.rs) | Embed an asset in the application binary and load it
[Extra asset source](../examples/asset/extra_source.rs) | Load an asset from a non-standard asset source
[Hot Reloading of Assets](../examples/asset/hot_asset_reloading.rs) | Demonstrates automatic reloading of assets when modified on disk
[Mult-asset synchronization](../examples/asset/multi_asset_sync.rs) | Demonstrates how to wait for multiple assets to be loaded.
[Repeated texture configuration](../examples/asset/repeated_texture.rs) | How to configure the texture to repeat instead of the default clamp to edges

## Async Tasks

Example | Description
--- | ---
[Async Compute](../examples/async_tasks/async_compute.rs) | How to use `AsyncComputeTaskPool` to complete longer running tasks
[External Source of Data on an External Thread](../examples/async_tasks/external_source_external_thread.rs) | How to use an external thread to run an infinite task and communicate with a channel

## Audio

Example | Description
--- | ---
[Audio](../examples/audio/audio.rs) | Shows how to load and play an audio file
[Audio Control](../examples/audio/audio_control.rs) | Shows how to load and play an audio file, and control how it's played
[Decodable](../examples/audio/decodable.rs) | Shows how to create and register a custom audio source by implementing the `Decodable` type.
[Pitch](../examples/audio/pitch.rs) | Shows how to directly play a simple pitch
[Soundtrack](../examples/audio/soundtrack.rs) | Shows how to play different soundtracks based on game state
[Spatial Audio 2D](../examples/audio/spatial_audio_2d.rs) | Shows how to play spatial audio, and moving the emitter in 2D
[Spatial Audio 3D](../examples/audio/spatial_audio_3d.rs) | Shows how to play spatial audio, and moving the emitter in 3D

## Camera

Example | Description
--- | ---
[2D top-down camera](../examples/camera/2d_top_down_camera.rs) | A 2D top-down camera smoothly following player movements
[First person view model](../examples/camera/first_person_view_model.rs) | A first-person camera that uses a world model and a view model with different field of views (FOV)

## Dev tools

Example | Description
--- | ---
[FPS overlay](../examples/dev_tools/fps_overlay.rs) | Demonstrates FPS overlay

## Diagnostics

Example | Description
--- | ---
[Custom Diagnostic](../examples/diagnostics/custom_diagnostic.rs) | Shows how to create a custom diagnostic
[Log Diagnostics](../examples/diagnostics/log_diagnostics.rs) | Add a plugin that logs diagnostics, like frames per second (FPS), to the console

## ECS (Entity Component System)

Example | Description
--- | ---
[Component Change Detection](../examples/ecs/component_change_detection.rs) | Change detection on components
[Component Hooks](../examples/ecs/component_hooks.rs) | Define component hooks to manage component lifecycle events
[Custom Query Parameters](../examples/ecs/custom_query_param.rs) | Groups commonly used compound queries and query filters into a single type
[Custom Schedule](../examples/ecs/custom_schedule.rs) | Demonstrates how to add custom schedules
[Dynamic ECS](../examples/ecs/dynamic.rs) | Dynamically create components, spawn entities with those components and query those components
[ECS Guide](../examples/ecs/ecs_guide.rs) | Full guide to Bevy's ECS
[Event](../examples/ecs/event.rs) | Illustrates event creation, activation, and reception
[Fixed Timestep](../examples/ecs/fixed_timestep.rs) | Shows how to create systems that run every fixed timestep, rather than every tick
[Generic System](../examples/ecs/generic_system.rs) | Shows how to create systems that can be reused with different types
[Hierarchy](../examples/ecs/hierarchy.rs) | Creates a hierarchy of parents and children entities
[Iter Combinations](../examples/ecs/iter_combinations.rs) | Shows how to iterate over combinations of query results
[Nondeterministic System Order](../examples/ecs/nondeterministic_system_order.rs) | Systems run in parallel, but their order isn't always deterministic. Here's how to detect and fix this.
[Observers](../examples/ecs/observers.rs) | Demonstrates observers that react to events (both built-in life-cycle events and custom events)
[One Shot Systems](../examples/ecs/one_shot_systems.rs) | Shows how to flexibly run systems without scheduling them
[Parallel Query](../examples/ecs/parallel_query.rs) | Illustrates parallel queries with `ParallelIterator`
[Removal Detection](../examples/ecs/removal_detection.rs) | Query for entities that had a specific component removed earlier in the current frame
[Run Conditions](../examples/ecs/run_conditions.rs) | Run systems only when one or multiple conditions are met
[Send and receive events](../examples/ecs/send_and_receive_events.rs) | Demonstrates how to send and receive events of the same type in a single system
[Startup System](../examples/ecs/startup_system.rs) | Demonstrates a startup system (one that runs once when the app starts up)
[System Closure](../examples/ecs/system_closure.rs) | Show how to use closures as systems, and how to configure `Local` variables by capturing external state
[System Parameter](../examples/ecs/system_param.rs) | Illustrates creating custom system parameters with `SystemParam`
[System Piping](../examples/ecs/system_piping.rs) | Pipe the output of one system into a second, allowing you to handle any errors gracefully
[System Stepping](../examples/ecs/system_stepping.rs) | Demonstrate stepping through systems in order of execution.

## Games

Example | Description
--- | ---
[Alien Cake Addict](../examples/games/alien_cake_addict.rs) | Eat the cakes. Eat them all. An example 3D game
[Breakout](../examples/games/breakout.rs) | An implementation of the classic game "Breakout".
[Contributors](../examples/games/contributors.rs) | Displays each contributor as a bouncy bevy-ball!
[Desk Toy](../examples/games/desk_toy.rs) | Bevy logo as a desk toy using transparent windows! Now with Googly Eyes!
[Game Menu](../examples/games/game_menu.rs) | A simple game menu
[Loading Screen](../examples/games/loading_screen.rs) | Demonstrates how to create a loading screen that waits for all assets to be loaded and render pipelines to be compiled.

## Gizmos

Example | Description
--- | ---
[2D Gizmos](../examples/gizmos/2d_gizmos.rs) | A scene showcasing 2D gizmos
[3D Gizmos](../examples/gizmos/3d_gizmos.rs) | A scene showcasing 3D gizmos
[Axes](../examples/gizmos/axes.rs) | Demonstrates the function of axes gizmos
[Light Gizmos](../examples/gizmos/light_gizmos.rs) | A scene showcasing light gizmos

## Input

Example | Description
--- | ---
[Char Input Events](../examples/input/char_input_events.rs) | Prints out all chars as they are inputted
[Gamepad Input](../examples/input/gamepad_input.rs) | Shows handling of gamepad input, connections, and disconnections
[Gamepad Input Events](../examples/input/gamepad_input_events.rs) | Iterates and prints gamepad input and connection events
[Gamepad Rumble](../examples/input/gamepad_rumble.rs) | Shows how to rumble a gamepad using force feedback
[Keyboard Input](../examples/input/keyboard_input.rs) | Demonstrates handling a key press/release
[Keyboard Input Events](../examples/input/keyboard_input_events.rs) | Prints out all keyboard events
[Keyboard Modifiers](../examples/input/keyboard_modifiers.rs) | Demonstrates using key modifiers (ctrl, shift)
[Mouse Grab](../examples/input/mouse_grab.rs) | Demonstrates how to grab the mouse, locking the cursor to the app's screen
[Mouse Input](../examples/input/mouse_input.rs) | Demonstrates handling a mouse button press/release
[Mouse Input Events](../examples/input/mouse_input_events.rs) | Prints out all mouse events (buttons, movement, etc.)
[Text Input](../examples/input/text_input.rs) | Simple text input with IME support
[Touch Input](../examples/input/touch_input.rs) | Displays touch presses, releases, and cancels
[Touch Input Events](../examples/input/touch_input_events.rs) | Prints out all touch inputs

## Math

Example | Description
--- | ---
[Custom Primitives](../examples/math/custom_primitives.rs) | Demonstrates how to add custom primitives and useful traits for them.
[Random Sampling](../examples/math/random_sampling.rs) | Demonstrates how to sample random points from mathematical primitives
[Rendering Primitives](../examples/math/render_primitives.rs) | Shows off rendering for all math primitives as both Meshes and Gizmos
[Sampling Primitives](../examples/math/sampling_primitives.rs) | Demonstrates all the primitives which can be sampled.

## Reflection

Example | Description
--- | ---
[Custom Attributes](../examples/reflection/custom_attributes.rs) | Registering and accessing custom attributes on reflected types
[Dynamic Types](../examples/reflection/dynamic_types.rs) | How dynamic types are used with reflection
[Generic Reflection](../examples/reflection/generic_reflection.rs) | Registers concrete instances of generic types that may be used with reflection
[Reflection](../examples/reflection/reflection.rs) | Demonstrates how reflection in Bevy provides a way to dynamically interact with Rust types
[Reflection Types](../examples/reflection/reflection_types.rs) | Illustrates the various reflection types available
[Trait Reflection](../examples/reflection/trait_reflection.rs) | Allows reflection with trait objects

## Scene

Example | Description
--- | ---
[Scene](../examples/scene/scene.rs) | Demonstrates loading from and saving scenes to files

## Shaders

These examples demonstrate how to implement different shaders in user code.

A shader in its most common usage is a small program that is run by the GPU per-vertex in a mesh (a vertex shader) or per-affected-screen-fragment (a fragment shader.) The GPU executes these programs in a highly parallel way.

There are also compute shaders which are used for more general processing leveraging the GPU's parallelism.

Example | Description
--- | ---
[Animated](../examples/shader/animate_shader.rs) | A shader that uses dynamic data like the time since startup
[Array Texture](../examples/shader/array_texture.rs) | A shader that shows how to reuse the core bevy PBR shading functionality in a custom material that obtains the base color from an array texture.
[Compute - Game of Life](../examples/shader/compute_shader_game_of_life.rs) | A compute shader that simulates Conway's Game of Life
[Custom Vertex Attribute](../examples/shader/custom_vertex_attribute.rs) | A shader that reads a mesh's custom vertex attribute
[Custom phase item](../examples/shader/custom_phase_item.rs) | Demonstrates how to enqueue custom draw commands in a render phase
[Extended Material](../examples/shader/extended_material.rs) | A custom shader that builds on the standard material
[GPU readback](../examples/shader/gpu_readback.rs) | A very simple compute shader that writes to a buffer that is read by the cpu
[Instancing](../examples/shader/shader_instancing.rs) | A shader that renders a mesh multiple times in one draw call
[Material](../examples/shader/shader_material.rs) | A shader and a material that uses it
[Material](../examples/shader/shader_material_2d.rs) | A shader and a material that uses it on a 2d mesh
[Material - GLSL](../examples/shader/shader_material_glsl.rs) | A shader that uses the GLSL shading language
[Material - Screenspace Texture](../examples/shader/shader_material_screenspace_texture.rs) | A shader that samples a texture with view-independent UV coordinates
[Material Prepass](../examples/shader/shader_prepass.rs) | A shader that uses the various textures generated by the prepass
[Post Processing - Custom Render Pass](../examples/shader/post_processing.rs) | A custom post processing effect, using a custom render pass that runs after the main pass
[Shader Defs](../examples/shader/shader_defs.rs) | A shader that uses "shaders defs" (a bevy tool to selectively toggle parts of a shader)
[Texture Binding Array (Bindless Textures)](../examples/shader/texture_binding_array.rs) | A shader that shows how to bind and sample multiple textures as a binding array (a.k.a. bindless textures).

## State

Example | Description
--- | ---
[Computed States](../examples/state/computed_states.rs) | Advanced state patterns using Computed States.
[Custom State Transition Behavior](../examples/state/custom_transitions.rs) | Creating and working with custom state transition schedules.
[States](../examples/state/states.rs) | Illustrates how to use States to control transitioning from a Menu state to an InGame state.
[Sub States](../examples/state/sub_states.rs) | Using Sub States for hierarchical state handling.

## Stress Tests

These examples are used to test the performance and stability of various parts of the engine in an isolated way.

Due to the focus on performance it's recommended to run the stress tests in release mode:

```sh
cargo run --release --example <example name>
```

Example | Description
--- | ---
[Bevymark](../examples/stress_tests/bevymark.rs) | A heavy sprite rendering workload to benchmark your system with Bevy
[Many Animated Sprites](../examples/stress_tests/many_animated_sprites.rs) | Displays many animated sprites in a grid arrangement with slight offsets to their animation timers. Used for performance testing.
[Many Buttons](../examples/stress_tests/many_buttons.rs) | Test rendering of many UI elements
[Many Cubes](../examples/stress_tests/many_cubes.rs) | Simple benchmark to test per-entity draw overhead. Run with the `sphere` argument to test frustum culling
[Many Foxes](../examples/stress_tests/many_foxes.rs) | Loads an animated fox model and spawns lots of them. Good for testing skinned mesh performance. Takes an unsigned integer argument for the number of foxes to spawn. Defaults to 1000
[Many Gizmos](../examples/stress_tests/many_gizmos.rs) | Test rendering of many gizmos
[Many Glyphs](../examples/stress_tests/many_glyphs.rs) | Simple benchmark to test text rendering.
[Many Lights](../examples/stress_tests/many_lights.rs) | Simple benchmark to test rendering many point lights. Run with `WGPU_SETTINGS_PRIO=webgl2` to restrict to uniform buffers and max 256 lights
[Many Sprites](../examples/stress_tests/many_sprites.rs) | Displays many sprites in a grid arrangement! Used for performance testing. Use `--colored` to enable color tinted sprites.
[Text Pipeline](../examples/stress_tests/text_pipeline.rs) | Text Pipeline benchmark
[Transform Hierarchy](../examples/stress_tests/transform_hierarchy.rs) | Various test cases for hierarchy and transform propagation performance

## Time

Example | Description
--- | ---
[Time handling](../examples/time/time.rs) | Explains how Time is handled in ECS
[Timers](../examples/time/timers.rs) | Illustrates ticking `Timer` resources inside systems and handling their state
[Virtual time](../examples/time/virtual_time.rs) | Shows how `Time<Virtual>` can be used to pause, resume, slow down and speed up a game.

## Tools

Example | Description
--- | ---
[Gamepad Viewer](../examples/tools/gamepad_viewer.rs) | Shows a visualization of gamepad buttons, sticks, and triggers
[Scene Viewer](../examples/tools/scene_viewer/main.rs) | A simple way to view glTF models with Bevy. Just run `cargo run --release --example scene_viewer /path/to/model.gltf#Scene0`, replacing the path as appropriate. With no arguments it will load the FieldHelmet glTF model from the repository assets subdirectory

## Transforms

Example | Description
--- | ---
[3D Rotation](../examples/transforms/3d_rotation.rs) | Illustrates how to (constantly) rotate an object around an axis
[Alignment](../examples/transforms/align.rs) | A demonstration of Transform's axis-alignment feature
[Scale](../examples/transforms/scale.rs) | Illustrates how to scale an object in each direction
[Transform](../examples/transforms/transform.rs) | Shows multiple transformations of objects
[Translation](../examples/transforms/translation.rs) | Illustrates how to move an object along an axis

## UI (User Interface)

Example | Description
--- | ---
[Borders](../examples/ui/borders.rs) | Demonstrates how to create a node with a border
[Button](../examples/ui/button.rs) | Illustrates creating and updating a button
[CSS Grid](../examples/ui/grid.rs) | An example for CSS Grid layout
[Display and Visibility](../examples/ui/display_and_visibility.rs) | Demonstrates how Display and Visibility work in the UI.
[Flex Layout](../examples/ui/flex_layout.rs) | Demonstrates how the AlignItems and JustifyContent properties can be composed to layout nodes and position text
[Font Atlas Debug](../examples/ui/font_atlas_debug.rs) | Illustrates how FontAtlases are populated (used to optimize text rendering internally)
[Overflow](../examples/ui/overflow.rs) | Simple example demonstrating overflow behavior
[Overflow and Clipping Debug](../examples/ui/overflow_debug.rs) | An example to debug overflow and clipping behavior
[Relative Cursor Position](../examples/ui/relative_cursor_position.rs) | Showcases the RelativeCursorPosition component
[Render UI to Texture](../examples/ui/render_ui_to_texture.rs) | An example of rendering UI as a part of a 3D world
[Rounded Borders](../examples/ui/rounded_borders.rs) | Demonstrates how to create a node with a rounded border
[Size Constraints](../examples/ui/size_constraints.rs) | Demonstrates how the to use the size constraints to control the size of a UI node.
[Text](../examples/ui/text.rs) | Illustrates creating and updating text
[Text Debug](../examples/ui/text_debug.rs) | An example for debugging text layout
[Text Wrap Debug](../examples/ui/text_wrap_debug.rs) | Demonstrates text wrapping
[Transparency UI](../examples/ui/transparency_ui.rs) | Demonstrates transparency for UI
[UI](../examples/ui/ui.rs) | Illustrates various features of Bevy UI
[UI Material](../examples/ui/ui_material.rs) | Demonstrates creating and using custom Ui materials
[UI Scaling](../examples/ui/ui_scaling.rs) | Illustrates how to scale the UI
[UI Texture Atlas](../examples/ui/ui_texture_atlas.rs) | Illustrates how to use TextureAtlases in UI
[UI Texture Atlas Slice](../examples/ui/ui_texture_atlas_slice.rs) | Illustrates how to use 9 Slicing for TextureAtlases in UI
[UI Texture Slice](../examples/ui/ui_texture_slice.rs) | Illustrates how to use 9 Slicing in UI
[UI Texture Slice Flipping and Tiling](../examples/ui/ui_texture_slice_flip_and_tile.rs) | Illustrates how to flip and tile images with 9 Slicing in UI
[UI Z-Index](../examples/ui/z_index.rs) | Demonstrates how to control the relative depth (z-position) of UI elements
[Viewport Debug](../examples/ui/viewport_debug.rs) | An example for debugging viewport coordinates
[Window Fallthrough](../examples/ui/window_fallthrough.rs) | Illustrates how to access `winit::window::Window`'s `hittest` functionality.

## Window

Example | Description
--- | ---
[Clear Color](../examples/window/clear_color.rs) | Creates a solid color window
[Custom User Event](../examples/window/custom_user_event.rs) | Handles custom user events within the event loop
[Low Power](../examples/window/low_power.rs) | Demonstrates settings to reduce power use for bevy applications
[Multiple Windows](../examples/window/multiple_windows.rs) | Demonstrates creating multiple windows, and rendering to them
[Scale Factor Override](../examples/window/scale_factor_override.rs) | Illustrates how to customize the default window settings
[Screenshot](../examples/window/screenshot.rs) | Shows how to save screenshots to disk
[Transparent Window](../examples/window/transparent_window.rs) | Illustrates making the window transparent and hiding the window decoration
[Window Resizing](../examples/window/window_resizing.rs) | Demonstrates resizing and responding to resizing a window
[Window Settings](../examples/window/window_settings.rs) | Demonstrates customizing default window settings

# Tests

Example | Description
--- | ---
[How to Test Systems](../tests/how_to_test_systems.rs) | How to test systems with commands, queries or resources

# Platform-Specific Examples

## Android

### Setup

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

### Build & Run

To run on a device setup for Android development, run:

```sh
cargo apk run -p bevy_mobile_example
```

When using Bevy as a library, the following fields must be added to `Cargo.toml`:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]

[package.metadata.android.sdk]
target_sdk_version = 31
```

Please reference `cargo-apk` [README](https://crates.io/crates/cargo-apk) for other Android Manifest fields.

### Debugging

You can view the logs with the following command:

```sh
adb logcat | grep 'RustStdoutStderr\|bevy\|wgpu'
```

In case of an error getting a GPU or setting it up, you can try settings logs of `wgpu_hal` to `DEBUG` to get more information.

Sometimes, running the app complains about an unknown activity. This may be fixed by uninstalling the application:

```sh
adb uninstall org.bevyengine.example
```

### Old phones

Bevy by default targets Android API level 31 in its examples which is the <!-- markdown-link-check-disable -->
[Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in Cargo.toml:

```toml
[package.metadata.android.sdk]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```

Example | File | Description
--- | --- | ---
`android` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

## iOS

### Setup

You need to install the correct rust targets:

- `aarch64-apple-ios`: iOS devices
- `x86_64-apple-ios`: iOS simulator on x86 processors
- `aarch64-apple-ios-sim`: iOS simulator on Apple processors

```sh
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

### Build & Run

Using bash:

```sh
cd examples/mobile
make run
```

In an ideal world, this will boot up, install and run the app for the first
iOS simulator in your `xcrun simctl list devices`. If this fails, you can
specify the simulator device UUID via:

```sh
DEVICE_ID=${YOUR_DEVICE_ID} make run
```

If you'd like to see xcode do stuff, you can run

```sh
open bevy_mobile_example.xcodeproj/
```

which will open xcode. You then must push the zoom zoom play button and wait
for the magic.

Example | File | Description
--- | --- | ---
`ios` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

## WASM

### Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

### Build & Run

Following is an example for `lighting`. For other examples, change the `lighting` in the
following commands.

```sh
cargo build --release --example lighting --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example \
  --out-dir examples/wasm/target \
  --target web target/wasm32-unknown-unknown/release/examples/lighting.wasm
```

The first command will build the example for the wasm target, creating a binary. Then,
[wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/reference/cli.html) is used to create
javascript bindings to this wasm file in the output file `examples/wasm/target/wasm_example.js`, which can be loaded using this
[example HTML file](./wasm/index.html).

Then serve `examples/wasm` directory to browser. i.e.

```sh
# cargo install basic-http-server
basic-http-server examples/wasm

# with python
python3 -m http.server --directory examples/wasm

# with ruby
ruby -run -ehttpd examples/wasm
```

#### WebGL2 and WebGPU

Bevy support for WebGPU is being worked on, but is currently experimental.

To build for WebGPU, you'll need to enable the `webgpu` feature. This will override the `webgl2` feature, and builds with the `webgpu` feature enabled won't be able to run on browsers that don't support WebGPU.

Bevy has an helper to build its examples:

- Build for WebGL2: `cargo run -p build-wasm-example -- --api webgl2 load_gltf`
- Build for WebGPU: `cargo run -p build-wasm-example -- --api webgpu load_gltf`

This helper will log the command used to build the examples.

### Audio in the browsers

For the moment, everything is single threaded, this can lead to stuttering when playing audio in browsers. Not all browsers react the same way for all games, you will have to experiment for your game.

In browsers, audio is not authorized to start without being triggered by an user interaction. This is to avoid multiple tabs all starting to auto play some sounds. You can find more context and explanation for this on [Google Chrome blog](https://developer.chrome.com/blog/web-audio-autoplay/). This page also describes a JS workaround to resume audio as soon as the user interact with your game.

### Optimizing

On the web, it's useful to reduce the size of the files that are distributed.
With rust, there are many ways to improve your executable sizes.
Here are some.

#### 1. Tweak your `Cargo.toml`

Add a new [profile](https://doc.rust-lang.org/cargo/reference/profiles.html)
to your `Cargo.toml`:

```toml
[profile.wasm-release]
# Use release profile as default values
inherits = "release"

# Optimize with size in mind, also try "s", sometimes it is better.
# This doesn't increase compilation times compared to -O3, great improvements
opt-level = "z"

# Do a second optimization pass removing duplicate or unused code from dependencies.
# Slows compile times, marginal improvements
lto = "fat"

# When building crates, optimize larger chunks at a time
# Slows compile times, marginal improvements
codegen-units = 1
```

Now, when building the final executable, use the `wasm-release` profile
by replacing `--release` by `--profile wasm-release` in the cargo command.

```sh
cargo build --profile wasm-release --example lighting --target wasm32-unknown-unknown
```

Make sure your final executable size is smaller, some of those optimizations
may not be worth keeping, due to compilation time increases.

#### 2. Use `wasm-opt` from the binaryen package

Binaryen is a set of tools for working with wasm. It has a `wasm-opt` CLI tool.

First download the `binaryen` package,
then locate the `.wasm` file generated by `wasm-bindgen`.
It should be in the `--out-dir` you specified in the command line,
the file name should end in `_bg.wasm`.

Then run `wasm-opt` with the `-Oz` flag. Note that `wasm-opt` is _very slow_.

Note that `wasm-opt` optimizations might not be as effective if you
didn't apply the optimizations from the previous section.

```sh
wasm-opt -Oz --output optimized.wasm examples/wasm/target/lighting_bg.wasm
mv optimized.wasm examples/wasm/target/lighting_bg.wasm
```

For a small project with a basic 3d model and two lights,
the generated file sizes are, as of July 2022, as follows:

|profile                           | wasm-opt | no wasm-opt |
|----------------------------------|----------|-------------|
|Default                           | 8.5M     | 13.0M       |
|opt-level = "z"                   | 6.1M     | 12.7M       |
|"z" + lto = "thin"                | 5.9M     | 12M         |
|"z" + lto = "fat"                 | 5.1M     | 9.4M        |
|"z" + "thin" + codegen-units = 1  | 5.3M     | 11M         |
|"z" + "fat"  + codegen-units = 1  | 4.8M     | 8.5M        |

There are more advanced optimization options available,
check the following pages for more info:

- <https://rustwasm.github.io/book/reference/code-size.html>
- <https://rustwasm.github.io/docs/wasm-bindgen/reference/optimize-size.html>
- <https://rustwasm.github.io/book/game-of-life/code-size.html>

### Loading Assets

To load assets, they need to be available in the folder examples/wasm/assets. Cloning this
repository will set it up as a symlink on Linux and macOS, but you will need to manually move
the assets on Windows.
