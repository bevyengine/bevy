---
title: Transform Overhaul
authors: ["@NthTensor"]
pull_requests: [TBD]
---

Bevy runs entirely on 3d math, even for 2d games. This is lets is simplify a lot of the internals, but it also forces users to deal with the complexity of Quaternion (3d rotations), and is just not very ergonomic in the 2d case.

So, in an effort to make 2d a bit nicer to work with, we've introduced a new `Transform2d` component, and renamed `Transform` to `Transform3d`. The new `Transform2d` component works kind of like a 3d transform which only allows positioning in the X-Y plane, and has a simple angle-based interface. Both components get propagated into `GlobalTransform`, and if you add both to the same entity you'll get an error.
