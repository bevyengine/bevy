---
title: "`Atmosphere` has been moved to `bevy_light`"
pull_requests: [22709]
---

If you were importing `Atmosphere`, `ScatteringMedium`, `ScatteringTerm`, `PhaseFunction`, or `Falloff` from `bevy_pbr`, they have been moved to live in `bevy_light` now.
`Atmosphere` is available at `bevy::light::Atmosphere`, the rest are under `bevy::light::atmosphere`.
