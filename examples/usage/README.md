# Usage Examples

See the [main examples README](../) for general information about Bevy's examples.

---

These examples demonstrate how to accomplish common game development tasks in Bevy.

---

The examples are grouped into categories for easier navigation:

- [Camera](#camera)
- [Control Flow](#control-flow)
- [Movement](#movement)

---

## Camera

Example | Description
--- | ---
[2D top-down camera](./camera/2d_top_down_camera.rs) | A 2D top-down camera smoothly following player movements
[2D Screen Shake](./camera/2d_screen_shake.rs) | A simple 2D screen shake effect
[3D Camera Orbit](./camera/camera_orbit.rs) | Shows how to orbit a static scene using pitch, yaw, and roll.
[First person view model](./camera/first_person_view_model.rs) | A first-person camera that uses a world model and a view model with different field of views (FOV)
[Split Screen](./camera/split_screen.rs) | Demonstrates how to render two cameras to the same window to accomplish "split screen"

## Control Flow

Example | Description
--- | ---
[Game Menu](./control_flow/game_menu.rs) | A simple "main menu"
[Loading Screen](./control_flow/loading_screen.rs) | Demonstrates how to create a loading screen that waits for all assets to be loaded and render pipelines to be compiled.

## Movement

Example | Description
--- | ---
[Run physics in a fixed timestep](../examples/movement/physics_in_fixed_timestep.rs) | Handles input, physics, and rendering in an industry-standard way by using a fixed timestep.
[Smooth Follow](../examples/movement/smooth_follow.rs) | Demonstrates how to make an entity smoothly follow another using interpolation
