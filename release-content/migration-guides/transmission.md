---
title: "Transmission has been moved to `bevy_pbr`"
pull_requests: [22687, 22706]
---

`Camera3d::screen_space_specular_transmission_steps` and `Camera3d::screen_space_specular_transmission_quality` have been pulled out into a separate component, `ScreenSpaceTransmission`, and put in `bevy_pbr`.

`ScreenSpaceTransmissionQuality` has been moved from `bevy_camera` to `bevy_pbr`.

`ScreenSpaceTransmissionQuality` is no longer a `Resource`.

`ViewTransmissionTexture` and `Transmissive3d` has been moved from `bevy_core_pipelines` to `bevy_pbr`.

`Node3d::MainTransmissivePass` is now initialized by `PbrPlugin`.
