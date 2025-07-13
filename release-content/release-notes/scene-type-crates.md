---
title: Define scenes without depending on bevy_render
authors: ["@atlv24"]
pull_requests: [19997, 19991, 20000, 19949, 19943, 19953]
---

It is now possible to use cameras, lights, and meshes without depending on the Bevy renderer. This makes it possible for 3rd party custom renderers to be drop-in replacements for rendering existing scenes.
