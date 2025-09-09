---
title: Polylines and Polygons are no longer const-generic
pull_requests: [ 20250 ]
---

`Polyline2d`, `Polyline3d`, `Polygon`, and `ConvexPolygon` are no longer const-generic and now implement `Meshable` for
direct mesh generation. These types now use `Vec` instead of arrays internally and will therefore allocate and are no
longer `no_std` compatible.

If you need these types to be `no_std` and/or const-generic, please file an issue explaining your use case
and we can consider creating fixed side-count polygon/polyline variants.
