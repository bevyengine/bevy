---
title: "`define_atomic_id` now lives in `bevy_utils`"
pull_requests: [22417]
---

`define_atomic_id` was moved out of `bevy_render` and into `bevy_utils`. If you were using `bevy::render::define_atomic_id`, you can update to `bevy::utils::define_atomic_id`.
