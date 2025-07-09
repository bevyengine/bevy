---
title: Extendable Gizmo-Primitives
authors: ["@lynn-lumen"]
pull_requests: [20049]
---

The `GizmoPrimitive3d` trait, which is used to draw primitives using gizmos can now be implemented for custom primitives. 
If you did not declare any such primitives, you may need to remove this trait as it will now be unused. 

If you did implement `GizmoPrimitive3d` for a custom `GizmoBuffer`-like struct, you will need to move your implementation for custom primitives to the primitive itself and use that implementation in a separate method on the previously mentioned struct.