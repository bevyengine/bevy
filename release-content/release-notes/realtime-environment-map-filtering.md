---
title: Realtime-filtered environment maps
authors: ["@mate-h"]
pull_requests: [19076]
---

An environment map needs to be processed to be able to support uses beyond a simple skybox,
such as reflections at different roughness levels, and ambient light contribution.
This process is called filtering, and can either be done ahead of time (prefiltering), or
in realtime, although at a reduced quality.

Bevy already supported prefiltering, but its not always possible to prefilter: sometimes,
you only gain access to an environment map at runtime, for whatever reason.
Typically this is from realtime reflection probes, but can also be from other sources
for example, from a procedural skybox.

Now, Bevy supports both modes of filtering!
Adding a `GeneratedEnvironmentMapLight` to a `Camera` entity lets you use any environment map
with Bevy's renderer, and enjoy all the benefits of prefiltering with none of the asset processing.

(TODO: Embed screenshot of realtime filtering)

Special thanks to @JMS55 for the feedback and @atlv24 for contributing and helping the PR get over the finish line!
