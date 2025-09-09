---
title: Realtime-filtered environment maps
authors: ["@mate-h"]
pull_requests: [19076, 20529]
---

(TODO: Embed screenshot of atmosphere-generated reflections)

An environment map needs to be processed to be able to support uses beyond a simple skybox,
such as reflections at different roughness levels, and ambient light contribution.
This process is called filtering, and can either be done ahead of time (prefiltering), or
in realtime, although at a reduced quality.

Bevy already supported prefiltering, but its not always possible to prefilter: sometimes,
your environment map is not available until runtime.
Typically this is from realtime reflection probes, but you might also, for example,
be using a procedural skybox.

Now, Bevy supports both modes of filtering!
Adding a `GeneratedEnvironmentMapLight` to a `Camera` entity lets you use any environment map
with Bevy's renderer, and enjoy all the benefits of prefiltering with none of the asset processing.

We've made sure works with our built-in atmosphere shader too.
To enable this, add the new component `AtmosphereEnvironmentMapLight` to the camera entity.

This is fully dynamic per-view effect: no pre-baked environment maps are needed.
However, please be aware that light probes are not yet supported.

Special thanks to @atlv24, @JMS55 and @ecoskey for reviews, feedback, and assistance.
