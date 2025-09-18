---
title: Rename `ScalingMode` to `SpriteScalingMode`
pull_requests: [21100]
---

In the previous release, both `bevy_sprite::sprite` and `bevy_camera::projection` defined an enum named `ScalingMode`, in violation of our one-namespace rule.

To resolve this, the `ScalingMode` enum from `bevy::sprite` has been renamed to `SpriteScalingMode`.
