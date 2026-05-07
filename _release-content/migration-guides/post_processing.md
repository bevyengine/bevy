---
title: Post Processing Split
pull_requests: [23098]
---

For both `Core2dSystems` and `Core3dSystems`, the `PostProcess` system set has been split into `EarlyPostProcess` and `PostProcess`.  2D now also has a `Prepass`.  Both systems have the following sets:

- `Prepass`
- `MainPass`
- `EarlyPostProcess`
- `PostProcess`
