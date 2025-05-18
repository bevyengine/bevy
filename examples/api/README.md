# API Examples

See the [main examples README](../) for general information about Bevy's examples.

---

These examples demonstrate the various features and APIs of Bevy and how to use them.

---

The examples are grouped into categories for easier navigation:

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
- [Embedded](#embedded)
- [Gizmos](#gizmos)
- [Helpers](#helpers)
- [Input](#input)
- [Math](#math)
- [Picking](#picking)
- [Reflection](#reflection)
- [Remote Protocol](#remote-protocol)
- [Scene](#scene)
- [Shaders](#shaders)
- [State](#state)
- [Time](#time)
- [Transforms](#transforms)
- [UI (User Interface)](#ui-user-interface)
- [Window](#window)

---

<!-- MD026 - Hello, World! looks better with the ! -->
<!-- markdownlint-disable-next-line MD026 -->
## The Bare Minimum: Hello, World!

Example | Description
--- | ---
[`hello_world.rs`](./hello_world.rs) | Runs a minimal example that outputs "hello world"

## 2D Rendering

Example | Description
--- | ---
[2D Bloom](./2d/bloom_2d.rs) | Illustrates bloom post-processing in 2d
[2D Rotation](./2d/rotation.rs) | Demonstrates rotating entities in 2D with quaternions
[2D Shapes](./2d/2d_shapes.rs) | Renders simple 2D primitive shapes like circles and polygons
[2D Viewport To World](./2d/2d_viewport_to_world.rs) | Demonstrates how to use the `Camera::viewport_to_world_2d` method with a dynamic viewport and camera.
[2D Wireframe](./2d/wireframe_2d.rs) | Showcases wireframes for 2d meshes
[Arc 2D Meshes](./2d/mesh2d_arcs.rs) | Demonstrates UV-mapping of the circular segment and sector primitives
[CPU Drawing](./2d/cpu_draw.rs) | Manually read/write the pixels of a texture
[Custom glTF vertex attribute 2D](./2d/custom_gltf_vertex_attribute.rs) | Renders a glTF mesh in 2D with a custom vertex attribute
[Manual Mesh 2D](./2d/mesh2d_manual.rs) | Renders a custom mesh "manually" with "mid-level" renderer apis
[Mesh 2D](./2d/mesh2d.rs) | Renders a 2d mesh
[Mesh 2D With Vertex Colors](./2d/mesh2d_vertex_color_texture.rs) | Renders a 2d mesh with vertex color attributes
[Mesh2d Alpha Mode](./2d/mesh2d_alpha_mode.rs) | Used to test alpha modes with mesh2d
[Mesh2d Repeated Texture](./2d/mesh2d_repeated_texture.rs) | Showcase of using `uv_transform` on the `ColorMaterial` of a `Mesh2d`
[Move Sprite](./2d/move_sprite.rs) | Changes the transform of a sprite
[Pixel Grid Snapping](./2d/pixel_grid_snap.rs) | Shows how to create graphics that snap to the pixel grid by rendering to a texture in 2D
[Sprite](./2d/sprite.rs) | Renders a sprite
[Sprite Animation](./2d/sprite_animation.rs) | Animates a sprite in response to an event
[Sprite Flipping](./2d/sprite_flipping.rs) | Renders a sprite flipped along an axis
[Sprite Scale](./2d/sprite_scale.rs) | Shows how a sprite can be scaled into a rectangle while keeping the aspect ratio
[Sprite Sheet](./2d/sprite_sheet.rs) | Renders an animated sprite
[Sprite Slice](./2d/sprite_slice.rs) | Showcases slicing sprites into sections that can be scaled independently via the 9-patch technique
[Sprite Tile](./2d/sprite_tile.rs) | Renders a sprite tiled in a grid
[Text 2D](./2d/text2d.rs) | Generates text in 2D
[Texture Atlas](./2d/texture_atlas.rs) | Generates a texture atlas (sprite sheet) from individual sprites
[Transparency in 2D](./2d/transparency_2d.rs) | Demonstrates transparency in 2d

## 3D Rendering

Example | Description
--- | ---
[3D Bloom](./3d/bloom_3d.rs) | Illustrates bloom configuration using HDR and emissive materials
[3D Scene](./3d/3d_scene.rs) | Simple 3D scene with basic shapes and lighting
[3D Shapes](./3d/3d_shapes.rs) | A scene showcasing the built-in 3D shapes
[3D Viewport To World](./3d/3d_viewport_to_world.rs) | Demonstrates how to use the `Camera::viewport_to_world` method
[Animated Material](./3d/animated_material.rs) | Shows how to animate material properties
[Anisotropy](./3d/anisotropy.rs) | Displays an example model with anisotropy
[Anti-aliasing](./3d/anti_aliasing.rs) | Compares different anti-aliasing methods
[Atmosphere](./3d/atmosphere.rs) | A scene showcasing pbr atmospheric scattering
[Atmospheric Fog](./3d/atmospheric_fog.rs) | A scene showcasing the atmospheric fog effect
[Auto Exposure](./3d/auto_exposure.rs) | A scene showcasing auto exposure
[Blend Modes](./3d/blend_modes.rs) | Showcases different blend modes
[Built-in postprocessing](./3d/post_processing.rs) | Demonstrates the built-in postprocessing features
[Camera sub view](./3d/camera_sub_view.rs) | Demonstrates using different sub view effects on a camera
[Clearcoat](./3d/clearcoat.rs) | Demonstrates the clearcoat PBR feature
[Clustered Decals](./3d/clustered_decals.rs) | Demonstrates clustered decals
[Color grading](./3d/color_grading.rs) | Demonstrates color grading
[Decal](./3d/decal.rs) | Decal rendering
[Deferred Rendering](./3d/deferred_rendering.rs) | Renders meshes with both forward and deferred pipelines
[Depth of field](./3d/depth_of_field.rs) | Demonstrates depth of field
[Edit Gltf Material](./3d/edit_material_on_gltf.rs) | Showcases changing materials of a Gltf after Scene spawn
[Fog](./3d/fog.rs) | A scene showcasing the distance fog effect
[Fog volumes](./3d/fog_volumes.rs) | Demonstrates fog volumes
[Generate Custom Mesh](./3d/generate_custom_mesh.rs) | Simple showcase of how to generate a custom mesh with a custom texture
[Irradiance Volumes](./3d/irradiance_volumes.rs) | Demonstrates irradiance volumes
[Lighting](./3d/lighting.rs) | Illustrates various lighting options in a simple scene
[Lightmaps](./3d/lightmaps.rs) | Rendering a scene with baked lightmaps
[Lines](./3d/lines.rs) | Create a custom material to draw 3d lines
[Load glTF](./3d/load_gltf.rs) | Loads and renders a glTF file as a scene
[Load glTF extras](./3d/load_gltf_extras.rs) | Loads and renders a glTF file as a scene, including the gltf extras
[Mesh Ray Cast](./3d/mesh_ray_cast.rs) | Demonstrates ray casting with the `MeshRayCast` system parameter
[Meshlet](./3d/meshlet.rs) | Meshlet rendering for dense high-poly scenes (experimental)
[Mixed lighting](./3d/mixed_lighting.rs) | Demonstrates how to combine baked and dynamic lighting
[Motion Blur](./3d/motion_blur.rs) | Demonstrates per-pixel motion blur
[Occlusion Culling](./3d/occlusion_culling.rs) | Demonstration of Occlusion Culling
[Order Independent Transparency](./3d/order_independent_transparency.rs) | Demonstrates how to use OIT
[Orthographic View](./3d/orthographic.rs) | Shows how to create a 3D orthographic view (for isometric-look in games or CAD applications)
[Parallax Mapping](./3d/parallax_mapping.rs) | Demonstrates use of a normal map and depth map for parallax mapping
[Parenting](./3d/parenting.rs) | Demonstrates parent->child relationships and relative transformations
[Percentage-closer soft shadows](./3d/pcss.rs) | Demonstrates percentage-closer soft shadows (PCSS)
[Physically Based Rendering](./3d/pbr.rs) | Demonstrates use of Physically Based Rendering (PBR) properties
[Query glTF primitives](./3d/query_gltf_primitives.rs) | Query primitives in a glTF scene
[Reflection Probes](./3d/reflection_probes.rs) | Demonstrates reflection probes
[Render to Texture](./3d/render_to_texture.rs) | Shows how to render to a texture, useful for mirrors, UI, or exporting images
[Rotate Environment Map](./3d/rotate_environment_map.rs) | Demonstrates how to rotate the skybox and the environment map simultaneously
[Screen Space Ambient Occlusion](./3d/ssao.rs) | A scene showcasing screen space ambient occlusion
[Screen Space Reflections](./3d/ssr.rs) | Demonstrates screen space reflections with water ripples
[Scrolling fog](./3d/scrolling_fog.rs) | Demonstrates how to create the effect of fog moving in the wind
[Shadow Biases](./3d/shadow_biases.rs) | Demonstrates how shadow biases affect shadows in a 3d scene
[Shadow Caster and Receiver](./3d/shadow_caster_receiver.rs) | Demonstrates how to prevent meshes from casting/receiving shadows in a 3d scene
[Skybox](./3d/skybox.rs) | Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats.
[Specular Tint](./3d/specular_tint.rs) | Demonstrates specular tints and maps
[Spherical Area Lights](./3d/spherical_area_lights.rs) | Demonstrates how point light radius values affect light behavior
[Spotlight](./3d/spotlight.rs) | Illustrates spot lights
[Texture](./3d/texture.rs) | Shows configuration of texture materials
[Tonemapping](./3d/tonemapping.rs) | Compares tonemapping options
[Transmission](./3d/transmission.rs) | Showcases light transmission in the PBR material
[Transparency in 3D](./3d/transparency_3d.rs) | Demonstrates transparency in 3d
[Two Passes](./3d/two_passes.rs) | Renders two 3d passes to the same window from different perspectives
[Update glTF Scene](./3d/update_gltf_scene.rs) | Update a scene from a glTF file, either by spawning the scene as a child of another entity, or by accessing the entities of the scene
[Vertex Colors](./3d/vertex_colors.rs) | Shows the use of vertex colors
[Visibility range](./3d/visibility_range.rs) | Demonstrates visibility ranges
[Volumetric fog](./3d/volumetric_fog.rs) | Demonstrates volumetric fog and lighting
[Wireframe](./3d/wireframe.rs) | Showcases wireframe rendering

## Animation

Example | Description
--- | ---
[Animated Mesh](./animation/animated_mesh.rs) | Plays an animation on a skinned glTF model of a fox
[Animated Mesh Control](./animation/animated_mesh_control.rs) | Plays an animation from a skinned glTF with keyboard controls
[Animated Mesh Events](./animation/animated_mesh_events.rs) | Plays an animation from a skinned glTF with events
[Animated Transform](./animation/animated_transform.rs) | Create and play an animation defined by code that operates on the `Transform` component
[Animated UI](./animation/animated_ui.rs) | Shows how to use animation clips to animate UI properties
[Animation Events](./animation/animation_events.rs) | Demonstrate how to use animation events
[Animation Graph](./animation/animation_graph.rs) | Blends multiple animations together with a graph
[Animation Masks](./animation/animation_masks.rs) | Demonstrates animation masks
[Color animation](./animation/color_animation.rs) | Demonstrates how to animate colors using mixing and splines in different color spaces
[Custom Skinned Mesh](./animation/custom_skinned_mesh.rs) | Skinned mesh example with mesh and joints data defined in code
[Eased Motion](./animation/eased_motion.rs) | Demonstrates the application of easing curves to animate an object
[Easing Functions](./animation/easing_functions.rs) | Showcases the built-in easing functions
[Morph Targets](./animation/morph_targets.rs) | Plays an animation from a glTF file with meshes with morph targets
[glTF Skinned Mesh](./animation/gltf_skinned_mesh.rs) | Skinned mesh example with mesh and joints data loaded from a glTF file

## Application

Example | Description
--- | ---
[Advanced log layers](./app/log_layers_ecs.rs) | Illustrate how to transfer data between log layers and Bevy's ECS
[Custom Loop](./app/custom_loop.rs) | Demonstrates how to create a custom runner (to update an app manually)
[Drag and Drop](./app/drag_and_drop.rs) | An example that shows how to handle drag and drop in an app
[Empty](./app/empty.rs) | An empty application (does nothing)
[Empty with Defaults](./app/empty_defaults.rs) | An empty application with default plugins
[Headless](./app/headless.rs) | An application that runs without default plugins
[Headless Renderer](./app/headless_renderer.rs) | An application that runs with no window, but renders into image file
[Log layers](./app/log_layers.rs) | Illustrate how to add custom log layers
[Logs](./app/logs.rs) | Illustrate how to use generate log output
[No Renderer](./app/no_renderer.rs) | An application that runs with default plugins and displays an empty window, but without an actual renderer
[Plugin](./app/plugin.rs) | Demonstrates the creation and registration of a custom plugin
[Plugin Group](./app/plugin_group.rs) | Demonstrates the creation and registration of a custom plugin group
[Return after Run](./app/return_after_run.rs) | Show how to return to main after the Bevy app has exited
[Thread Pool Resources](./app/thread_pool_resources.rs) | Creates and customizes the internal thread pool
[Without Winit](./app/without_winit.rs) | Create an application without winit (runs single time, no event loop)

## Assets

Example | Description
--- | ---
[Alter Mesh](./asset/alter_mesh.rs) | Shows how to modify the underlying asset of a Mesh after spawning.
[Alter Sprite](./asset/alter_sprite.rs) | Shows how to modify texture assets after spawning.
[Asset Decompression](./asset/asset_decompression.rs) | Demonstrates loading a compressed asset
[Asset Loading](./asset/asset_loading.rs) | Demonstrates various methods to load assets
[Asset Processing](./asset/processing/asset_processing.rs) | Demonstrates how to process and load custom assets
[Asset Settings](./asset/asset_settings.rs) | Demonstrates various methods of applying settings when loading an asset
[Custom Asset](./asset/custom_asset.rs) | Implements a custom asset loader
[Custom Asset IO](./asset/custom_asset_reader.rs) | Implements a custom AssetReader
[Embedded Asset](./asset/embedded_asset.rs) | Embed an asset in the application binary and load it
[Extra asset source](./asset/extra_source.rs) | Load an asset from a non-standard asset source
[Hot Reloading of Assets](./asset/hot_asset_reloading.rs) | Demonstrates automatic reloading of assets when modified on disk
[Multi-asset synchronization](./asset/multi_asset_sync.rs) | Demonstrates how to wait for multiple assets to be loaded.
[Repeated texture configuration](./asset/repeated_texture.rs) | How to configure the texture to repeat instead of the default clamp to edges

## Async Tasks

Example | Description
--- | ---
[Async Compute](./async_tasks/async_compute.rs) | How to use `AsyncComputeTaskPool` to complete longer running tasks
[External Source of Data on an External Thread](./async_tasks/external_source_external_thread.rs) | How to use an external thread to run an infinite task and communicate with a channel

## Audio

Example | Description
--- | ---
[Audio](./audio/audio.rs) | Shows how to load and play an audio file
[Audio Control](./audio/audio_control.rs) | Shows how to load and play an audio file, and control how it's played
[Decodable](./audio/decodable.rs) | Shows how to create and register a custom audio source by implementing the `Decodable` type.
[Pitch](./audio/pitch.rs) | Shows how to directly play a simple pitch
[Soundtrack](./audio/soundtrack.rs) | Shows how to play different soundtracks based on game state
[Spatial Audio 2D](./audio/spatial_audio_2d.rs) | Shows how to play spatial audio, and moving the emitter in 2D
[Spatial Audio 3D](./audio/spatial_audio_3d.rs) | Shows how to play spatial audio, and moving the emitter in 3D

## Camera

Example | Description
--- | ---
[Custom Projection](./camera/custom_projection.rs) | Shows how to create custom camera projections.
[Projection Zoom](./camera/projection_zoom.rs) | Shows how to zoom orthographic and perspective projection cameras.

## Dev tools

Example | Description
--- | ---
[FPS overlay](./dev_tools/fps_overlay.rs) | Demonstrates FPS overlay

## Diagnostics

Example | Description
--- | ---
[Custom Diagnostic](./diagnostics/custom_diagnostic.rs) | Shows how to create a custom diagnostic
[Enabling/disabling diagnostic](./diagnostics/enabling_disabling_diagnostic.rs) | Shows how to disable/re-enable a Diagnostic during runtime
[Log Diagnostics](./diagnostics/log_diagnostics.rs) | Add a plugin that logs diagnostics, like frames per second (FPS), to the console

## ECS (Entity Component System)

Example | Description
--- | ---
[Change Detection](./ecs/change_detection.rs) | Change detection on components and resources
[Component Hooks](./ecs/component_hooks.rs) | Define component hooks to manage component lifecycle events
[Custom Query Parameters](./ecs/custom_query_param.rs) | Groups commonly used compound queries and query filters into a single type
[Custom Schedule](./ecs/custom_schedule.rs) | Demonstrates how to add custom schedules
[Dynamic ECS](./ecs/dynamic.rs) | Dynamically create components, spawn entities with those components and query those components
[ECS Guide](./ecs/ecs_guide.rs) | Full guide to Bevy's ECS
[Entity disabling](./ecs/entity_disabling.rs) | Demonstrates how to hide entities from the ECS without deleting them
[Error handling](./ecs/error_handling.rs) | How to return and handle errors across the ECS
[Event](./ecs/event.rs) | Illustrates event creation, activation, and reception
[Fallible System Parameters](./ecs/fallible_params.rs) | Systems are skipped if their parameters cannot be acquired
[Fixed Timestep](./ecs/fixed_timestep.rs) | Shows how to create systems that run every fixed timestep, rather than every tick
[Generic System](./ecs/generic_system.rs) | Shows how to create systems that can be reused with different types
[Hierarchy](./ecs/hierarchy.rs) | Creates a hierarchy of parents and children entities
[Immutable Components](./ecs/immutable_components.rs) | Demonstrates the creation and utility of immutable components
[Iter Combinations](./ecs/iter_combinations.rs) | Shows how to iterate over combinations of query results
[Nondeterministic System Order](./ecs/nondeterministic_system_order.rs) | Systems run in parallel, but their order isn't always deterministic. Here's how to detect and fix this.
[Observer Propagation](./ecs/observer_propagation.rs) | Demonstrates event propagation with observers
[Observers](./ecs/observers.rs) | Demonstrates observers that react to events (both built-in life-cycle events and custom events)
[One Shot Systems](./ecs/one_shot_systems.rs) | Shows how to flexibly run systems without scheduling them
[Parallel Query](./ecs/parallel_query.rs) | Illustrates parallel queries with `ParallelIterator`
[Relationships](./ecs/relationships.rs) | Define and work with custom relationships between entities
[Removal Detection](./ecs/removal_detection.rs) | Query for entities that had a specific component removed earlier in the current frame
[Run Conditions](./ecs/run_conditions.rs) | Run systems only when one or multiple conditions are met
[Send and receive events](./ecs/send_and_receive_events.rs) | Demonstrates how to send and receive events of the same type in a single system
[Startup System](./ecs/startup_system.rs) | Demonstrates a startup system (one that runs once when the app starts up)
[State Scoped](./ecs/state_scoped.rs) | Shows how to spawn entities that are automatically despawned either when entering or exiting specific game states.
[System Closure](./ecs/system_closure.rs) | Show how to use closures as systems, and how to configure `Local` variables by capturing external state
[System Parameter](./ecs/system_param.rs) | Illustrates creating custom system parameters with `SystemParam`
[System Piping](./ecs/system_piping.rs) | Pipe the output of one system into a second, allowing you to handle any errors gracefully
[System Stepping](./ecs/system_stepping.rs) | Demonstrate stepping through systems in order of execution.

## Embedded

Example | Description
--- | ---
[`no_std` Compatible Library](./no_std/library/src/lib.rs) | Example library compatible with `std` and `no_std` targets

## Gizmos

Example | Description
--- | ---
[2D Gizmos](./gizmos/2d_gizmos.rs) | A scene showcasing 2D gizmos
[3D Gizmos](./gizmos/3d_gizmos.rs) | A scene showcasing 3D gizmos
[Axes](./gizmos/axes.rs) | Demonstrates the function of axes gizmos
[Light Gizmos](./gizmos/light_gizmos.rs) | A scene showcasing light gizmos

## Helpers

Example | Description
--- | ---
[Camera Controller](./helpers/camera_controller.rs) | Example Free-Cam Styled Camera Controller
[Widgets](./helpers/widgets.rs) | Example UI Widgets

## Input

Example | Description
--- | ---
[Char Input Events](./input/char_input_events.rs) | Prints out all chars as they are inputted
[Gamepad Input](./input/gamepad_input.rs) | Shows handling of gamepad input, connections, and disconnections
[Gamepad Input Events](./input/gamepad_input_events.rs) | Iterates and prints gamepad input and connection events
[Gamepad Rumble](./input/gamepad_rumble.rs) | Shows how to rumble a gamepad using force feedback
[Keyboard Input](./input/keyboard_input.rs) | Demonstrates handling a key press/release
[Keyboard Input Events](./input/keyboard_input_events.rs) | Prints out all keyboard events
[Keyboard Modifiers](./input/keyboard_modifiers.rs) | Demonstrates using key modifiers (ctrl, shift)
[Mouse Grab](./input/mouse_grab.rs) | Demonstrates how to grab the mouse, locking the cursor to the app's screen
[Mouse Input](./input/mouse_input.rs) | Demonstrates handling a mouse button press/release
[Mouse Input Events](./input/mouse_input_events.rs) | Prints out all mouse events (buttons, movement, etc.)
[Text Input](./input/text_input.rs) | Simple text input with IME support
[Touch Input](./input/touch_input.rs) | Displays touch presses, releases, and cancels
[Touch Input Events](./input/touch_input_events.rs) | Prints out all touch inputs

## Math

Example | Description
--- | ---
[Bounding Volume Intersections (2D)](./math/bounding_2d.rs) | Showcases bounding volumes and intersection tests
[Cubic Splines](./math/cubic_splines.rs) | Exhibits different modes of constructing cubic curves using splines
[Custom Primitives](./math/custom_primitives.rs) | Demonstrates how to add custom primitives and useful traits for them.
[Random Sampling](./math/random_sampling.rs) | Demonstrates how to sample random points from mathematical primitives
[Rendering Primitives](./math/render_primitives.rs) | Shows off rendering for all math primitives as both Meshes and Gizmos
[Sampling Primitives](./math/sampling_primitives.rs) | Demonstrates all the primitives which can be sampled.

## Picking

Example | Description
--- | ---
[Mesh Picking](./picking/mesh_picking.rs) | Demonstrates picking meshes
[Picking Debug Tools](./picking/debug_picking.rs) | Demonstrates picking debug overlay
[Showcases simple picking events and usage](./picking/simple_picking.rs) | Demonstrates how to use picking events to spawn simple objects
[Sprite Picking](./picking/sprite_picking.rs) | Demonstrates picking sprites and sprite atlases

## Reflection

Example | Description
--- | ---
[Custom Attributes](./reflection/custom_attributes.rs) | Registering and accessing custom attributes on reflected types
[Dynamic Types](./reflection/dynamic_types.rs) | How dynamic types are used with reflection
[Function Reflection](./reflection/function_reflection.rs) | Demonstrates how functions can be called dynamically using reflection
[Generic Reflection](./reflection/generic_reflection.rs) | Registers concrete instances of generic types that may be used with reflection
[Reflection](./reflection/reflection.rs) | Demonstrates how reflection in Bevy provides a way to dynamically interact with Rust types
[Reflection Types](./reflection/reflection_types.rs) | Illustrates the various reflection types available
[Type Data](./reflection/type_data.rs) | Demonstrates how to create and use type data

## Remote Protocol

Example | Description
--- | ---
[client](./remote/client.rs) | A simple command line client that can control Bevy apps via the BRP
[server](./remote/server.rs) | A Bevy app that you can connect to with the BRP and edit

## Scene

Example | Description
--- | ---
[Scene](./scene/scene.rs) | Demonstrates loading from and saving scenes to files

## Shaders

These examples demonstrate how to implement different shaders in user code.

A shader in its most common usage is a small program that is run by the GPU per-vertex in a mesh (a vertex shader) or per-affected-screen-fragment (a fragment shader.) The GPU executes these programs in a highly parallel way.

There are also compute shaders which are used for more general processing leveraging the GPU's parallelism.

Example | Description
--- | ---
[Animated](./shader/animate_shader.rs) | A shader that uses dynamic data like the time since startup
[Array Texture](./shader/array_texture.rs) | A shader that shows how to reuse the core bevy PBR shading functionality in a custom material that obtains the base color from an array texture.
[Compute - Game of Life](./shader/compute_shader_game_of_life.rs) | A compute shader that simulates Conway's Game of Life
[Custom Render Phase](./shader/custom_render_phase.rs) | Shows how to make a complete render phase
[Custom Vertex Attribute](./shader/custom_vertex_attribute.rs) | A shader that reads a mesh's custom vertex attribute
[Custom phase item](./shader/custom_phase_item.rs) | Demonstrates how to enqueue custom draw commands in a render phase
[Extended Bindless Material](./shader/extended_material_bindless.rs) | Demonstrates bindless `ExtendedMaterial`
[Extended Material](./shader/extended_material.rs) | A custom shader that builds on the standard material
[GPU readback](./shader/gpu_readback.rs) | A very simple compute shader that writes to a buffer that is read by the cpu
[Instancing](./shader/custom_shader_instancing.rs) | A shader that renders a mesh multiple times in one draw call using low level rendering api
[Instancing](./shader/automatic_instancing.rs) | Shows that multiple instances of a cube are automatically instanced in one draw call
[Material](./shader/shader_material.rs) | A shader and a material that uses it
[Material](./shader/shader_material_2d.rs) | A shader and a material that uses it on a 2d mesh
[Material - Bindless](./shader/shader_material_bindless.rs) | Demonstrates how to make materials that use bindless textures
[Material - GLSL](./shader/shader_material_glsl.rs) | A shader that uses the GLSL shading language
[Material - Screenspace Texture](./shader/shader_material_screenspace_texture.rs) | A shader that samples a texture with view-independent UV coordinates
[Material - WESL](./shader/shader_material_wesl.rs) | A shader that uses WESL
[Material Prepass](./shader/shader_prepass.rs) | A shader that uses the various textures generated by the prepass
[Post Processing - Custom Render Pass](./shader/custom_post_processing.rs) | A custom post processing effect, using a custom render pass that runs after the main pass
[Shader Defs](./shader/shader_defs.rs) | A shader that uses "shaders defs" (a bevy tool to selectively toggle parts of a shader)
[Specialized Mesh Pipeline](./shader/specialized_mesh_pipeline.rs) | Demonstrates how to write a specialized mesh pipeline
[Storage Buffer](./shader/storage_buffer.rs) | A shader that shows how to bind a storage buffer using a custom material.
[Texture Binding Array (Bindless Textures)](./shader/texture_binding_array.rs) | A shader that shows how to bind and sample multiple textures as a binding array (a.k.a. bindless textures).

## State

Example | Description
--- | ---
[Computed States](./state/computed_states.rs) | Advanced state patterns using Computed States.
[Custom State Transition Behavior](./state/custom_transitions.rs) | Creating and working with custom state transition schedules.
[States](./state/states.rs) | Illustrates how to use States to control transitioning from a Menu state to an InGame state.
[Sub States](./state/sub_states.rs) | Using Sub States for hierarchical state handling.

## Time

Example | Description
--- | ---
[Time handling](./time/time.rs) | Explains how Time is handled in ECS
[Timers](./time/timers.rs) | Illustrates ticking `Timer` resources inside systems and handling their state
[Virtual time](./time/virtual_time.rs) | Shows how `Time<Virtual>` can be used to pause, resume, slow down and speed up a game.

## Transforms

Example | Description
--- | ---
[3D Rotation](./transforms/3d_rotation.rs) | Illustrates how to (constantly) rotate an object around an axis
[Alignment](./transforms/align.rs) | A demonstration of Transform's axis-alignment feature
[Scale](./transforms/scale.rs) | Illustrates how to scale an object in each direction
[Transform](./transforms/transform.rs) | Shows multiple transformations of objects
[Translation](./transforms/translation.rs) | Illustrates how to move an object along an axis

## UI (User Interface)

Example | Description
--- | ---
[Borders](./ui/borders.rs) | Demonstrates how to create a node with a border
[Box Shadow](./ui/box_shadow.rs) | Demonstrates how to create a node with a shadow
[Button](./ui/button.rs) | Illustrates creating and updating a button
[CSS Grid](./ui/grid.rs) | An example for CSS Grid layout
[Directional Navigation](./ui/directional_navigation.rs) | Demonstration of Directional Navigation between UI elements
[Display and Visibility](./ui/display_and_visibility.rs) | Demonstrates how Display and Visibility work in the UI.
[Flex Layout](./ui/flex_layout.rs) | Demonstrates how the AlignItems and JustifyContent properties can be composed to layout nodes and position text
[Font Atlas Debug](./ui/font_atlas_debug.rs) | Illustrates how FontAtlases are populated (used to optimize text rendering internally)
[Ghost Nodes](./ui/ghost_nodes.rs) | Demonstrates the use of Ghost Nodes to skip entities in the UI layout hierarchy
[Overflow](./ui/overflow.rs) | Simple example demonstrating overflow behavior
[Overflow Clip Margin](./ui/overflow_clip_margin.rs) | Simple example demonstrating the OverflowClipMargin style property
[Overflow and Clipping Debug](./ui/overflow_debug.rs) | An example to debug overflow and clipping behavior
[Relative Cursor Position](./ui/relative_cursor_position.rs) | Showcases the RelativeCursorPosition component
[Render UI to Texture](./ui/render_ui_to_texture.rs) | An example of rendering UI as a part of a 3D world
[Scroll](./ui/scroll.rs) | Demonstrates scrolling UI containers
[Size Constraints](./ui/size_constraints.rs) | Demonstrates how the to use the size constraints to control the size of a UI node.
[Tab Navigation](./ui/tab_navigation.rs) | Demonstration of Tab Navigation between UI elements
[Text](./ui/text.rs) | Illustrates creating and updating text
[Text Background Colors](./ui/text_background_colors.rs) | Demonstrates text background colors
[Text Debug](./ui/text_debug.rs) | An example for debugging text layout
[Text Wrap Debug](./ui/text_wrap_debug.rs) | Demonstrates text wrapping
[Transparency UI](./ui/transparency_ui.rs) | Demonstrates transparency for UI
[UI Material](./ui/ui_material.rs) | Demonstrates creating and using custom Ui materials
[UI Scaling](./ui/ui_scaling.rs) | Illustrates how to scale the UI
[UI Texture Atlas](./ui/ui_texture_atlas.rs) | Illustrates how to use TextureAtlases in UI
[UI Texture Atlas Slice](./ui/ui_texture_atlas_slice.rs) | Illustrates how to use 9 Slicing for TextureAtlases in UI
[UI Texture Slice](./ui/ui_texture_slice.rs) | Illustrates how to use 9 Slicing in UI
[UI Texture Slice Flipping and Tiling](./ui/ui_texture_slice_flip_and_tile.rs) | Illustrates how to flip and tile images with 9 Slicing in UI
[UI Z-Index](./ui/z_index.rs) | Demonstrates how to control the relative depth (z-position) of UI elements
[Viewport Debug](./ui/viewport_debug.rs) | An example for debugging viewport coordinates
[Viewport Node](./ui/viewport_node.rs) | Demonstrates how to create a viewport node with picking support
[Window Fallthrough](./ui/window_fallthrough.rs) | Illustrates how to access `winit::window::Window`'s `hittest` functionality.

## Window

Example | Description
--- | ---
[Clear Color](./window/clear_color.rs) | Creates a solid color window
[Custom Cursor Image](./window/custom_cursor_image.rs) | Demonstrates creating an animated custom cursor from an image
[Custom User Event](./window/custom_user_event.rs) | Handles custom user events within the event loop
[Low Power](./window/low_power.rs) | Demonstrates settings to reduce power use for bevy applications
[Monitor info](./window/monitor_info.rs) | Displays information about available monitors (displays).
[Multiple Windows](./window/multiple_windows.rs) | Demonstrates creating multiple windows, and rendering to them
[Scale Factor Override](./window/scale_factor_override.rs) | Illustrates how to customize the default window settings
[Screenshot](./window/screenshot.rs) | Shows how to save screenshots to disk
[Transparent Window](./window/transparent_window.rs) | Illustrates making the window transparent and hiding the window decoration
[Window Drag Move](./window/window_drag_move.rs) | Demonstrates drag move and drag resize without window decoration
[Window Resizing](./window/window_resizing.rs) | Demonstrates resizing and responding to resizing a window
[Window Settings](./window/window_settings.rs) | Demonstrates customizing default window settings
