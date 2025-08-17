---
title: Define scenes without depending on bevy_render
authors: ["@atlv24"]
pull_requests: [20502, 20498, 20485, 20496, 20493, 20492, 20491, 20488, 20487, 20486, 20483, 20480, 20479, 20478, 20477, 20473, 20472, 20471, 20470, 20392, 20390, 20388, 20345, 20344, 20330, 20051, 20000, 19997, 19991, 19985, 19973, 19965, 19963, 19962, 19960, 19959, 19958, 19957, 19956, 19955, 19954, 19953, 19949, 19943, 16620, 16619, 15700, 15666, 15650]
---

It is now possible to use cameras, lights, shaders, images, and meshes without depending on the Bevy renderer. This makes it possible for 3rd party custom renderers to be drop-in replacements for rendering existing scenes.
