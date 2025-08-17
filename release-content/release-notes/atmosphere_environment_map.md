---
title: Generated environment map for procedural atmosphere
authors: ["@mate-h"]
pull_requests: [20529]
---

(TODO: Embed screenshot of atmosphere-generated reflections)

You can now have dynamic reflections and ambient light in your scene that match the procedural sky.

As the sky changes, reflections on shiny and rough materials update automatically to stay consistent—no pre-baked environment maps needed.

To enable this for a camera, add the new component `AtmosphereEnvironmentMapLight` to the camera entity:

```rust
commands.spawn((
    Camera3d::default(),
    // Generates an environment cubemap from the atmosphere for this view
    AtmosphereEnvironmentMapLight::default(),
));
```

Notes:

- Negligible cost for the atmosphere cubemap pass, filtering adds a small overhead per view.
- Currently tied to the active view, scene light probes are not yet supported.

Special thanks to @atlv24, @JMS55 and @ecoskey for reviews and feedback.
