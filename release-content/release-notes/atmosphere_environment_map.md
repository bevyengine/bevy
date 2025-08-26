---
title: Generate environment maps from a procedural atmosphere
authors: ["@mate-h"]
pull_requests: [20529]
---

(TODO: Embed screenshot of atmosphere-generated reflections)

As your procedural sky changes, the reflections and ambient light in your scene should automatically be updated to match.
Now, this just works!
This is fully dynamic: no pre-baked environment maps are needed.

To enable this, add the new component `AtmosphereEnvironmentMapLight` to the camera entity:

```rust
commands.spawn((
    Camera3d::default(),
    // Generates an environment cubemap from the atmosphere for this view
    AtmosphereEnvironmentMapLight::default(),
));
```

Note that this is a per-view effect (per camera). Light probes are not yet supported.

Special thanks to @atlv24, @JMS55 and @ecoskey for reviews and feedback.
