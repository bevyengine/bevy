---
title: ScheduleBuildSettings now includes a shuffle_seed field.
pull_requests: [25094]
---

`ScheduleBuildSettings` now includes an additional `shuffle_seed` field if the `debug` feature is
enabled on `bevy` or `bevy_ecs`. Set this to `None` if you are exhaustively listing out fields.
