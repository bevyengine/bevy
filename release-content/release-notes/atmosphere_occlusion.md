---
title: "Atmosphere Occlusion and PBR Shading"
authors: ["@mate-h"]
pull_requests: [21383]
---

The procedural atmosphere now affects how light reaches objects in your scene! Sunlight automatically picks up the right colors as it travels through the atmosphere, appearing orange or red when the sun is closer to the horizon.

This works seamlessly with volumetric fog and all rendering modes, so your scenes will have more cohesive and realistic lighting right out of the box.

Check out the updated `atmosphere` example to see it in action!
