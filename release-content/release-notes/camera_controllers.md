---
title: First-party camera controllers
authors: ["@alice-i-cecile"]
pull_requests: [20215, 21450]
---

To understand a scene, you must look at it through the lens of a camera: explore it, and interact with it.
Because this is such a fundamental operation, game devs have developed a rich collection of tools
called "camera controllers" for manipulating them.

Getting camera controllers feeling *right* is both tricky and essential: they have a serious
impact on both the feeling of your game and the usability of your software.

Historically, Bevy has left this entirely up to individual game developers:
camera controllers require deep customization and endless twiddling.
However, Bevy as a game engine needs its *own* camera controllers:
allowing users to quickly and easily explore scenes during development (rather than gameplay).

To that end, we've created `bevy_camera_controller`: giving us a place to store, share and refine the camera controllers
that we need for easy development, and yes, an eventual Editor.
We're kicking it off with a couple of camera controllers, detailed below.

### `FreeCam`

The first camera controller that we've introduced is a "free cam", designed for quickly moving around a scene,
completely ignoring both physics and geometry.
You may have heard of a "flycam" controller before, which is a specialization of a "free cam" controller
designed for fast and fluid movement for covering large amounts of terrain.

To add a free cam controller to your project (typically under a `dev_mode` feature flag),
add the `FreeCamPlugin` and the `FreeCam` component to your camera entity.

To configure the settings (speed, behavior, keybindings) or enable / disable the controller modify the `FreeCam` component.
We've done our best to select good defaults, but the details of your scene (especially the scale!) will make a big
difference to what feels right.

### Using `bevy_camera_controller` in your own projects

The provided camera controllers are designed to be functional, pleasant debug and dev tools:
add the correct plugin and camera component and you're good to go!

They can also be useful for prototyping, giving you a quick-and-dirty camera controller
as you get your game off the ground.

However, they are deliberately *not* architected to give you the level of extensibility and customization
needed to make a production-grade camera controller for games.
Customizatibility comes with a real cost in terms of user experience and maintainability,
and because each project only ever needs one or two distinct camera controllers, exposing more knobs and levers is often a questionable design.
Instead, consider vendoring (read: copy-pasting the source code) the camera controller you want to extend
into your project and rewriting the quite-approachable logic to meet your needs,
or looking for [ecosystem camera crates](https://bevy.org/assets/#camera) that correspond to the genre you're building in.
