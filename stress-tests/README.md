# Stress Tests

These examples are used to stress test Bevy's performance in various ways. These
should be run with the "stress-test" profile to accurately represent performance
in production, otherwise they will run in cargo's default "dev" profile which is
very slow.

## Example Command

```bash
cargo run --profile stress-test --example <EXAMPLE>
```

---

Example | Description
--- | ---
[Bevymark](./bevymark.rs) | A heavy sprite rendering workload to benchmark your system with Bevy
[Many Animated Materials](./many_materials.rs) | Benchmark to test rendering many animated materials
[Many Animated Sprites](./many_animated_sprites.rs) | Displays many animated sprites in a grid arrangement with slight offsets to their animation timers. Used for performance testing.
[Many Buttons](./many_buttons.rs) | Test rendering of many UI elements
[Many Cameras & Lights](./many_cameras_lights.rs) | Test rendering of many cameras and lights
[Many Components (and Entities and Systems)](./many_components.rs) | Test large ECS systems
[Many Cubes](./many_cubes.rs) | Simple benchmark to test per-entity draw overhead. Run with the `sphere` argument to test frustum culling
[Many Foxes](./many_foxes.rs) | Loads an animated fox model and spawns lots of them. Good for testing skinned mesh performance. Takes an unsigned integer argument for the number of foxes to spawn. Defaults to 1000
[Many Gizmos](./many_gizmos.rs) | Test rendering of many gizmos
[Many Glyphs](./many_glyphs.rs) | Simple benchmark to test text rendering.
[Many Lights](./many_lights.rs) | Simple benchmark to test rendering many point lights. Run with `WGPU_SETTINGS_PRIO=webgl2` to restrict to uniform buffers and max 256 lights
[Many Sprites](./many_sprites.rs) | Displays many sprites in a grid arrangement! Used for performance testing. Use `--colored` to enable color tinted sprites.
[Many Text2d](./many_text2d.rs) | Displays many Text2d! Used for performance testing.
[Text Pipeline](./text_pipeline.rs) | Text Pipeline benchmark
[Transform Hierarchy](./transform_hierarchy.rs) | Various test cases for hierarchy and transform propagation performance

