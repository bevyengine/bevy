---
title: "Post process effects now live in `bevy_post_process`"
pull_requests: []
---

`bevy_core_pipeline` used to be home to many non-core things, including post process effects.
They have now been given a new home in `bevy_post_process`.

If you were importing Bloom, AutoExposure, ChromaticAberration, DepthOfField, or MotionBlur from `bevy_core_pipeline` or `bevy::core_pipeline`, you must now import them from `bevy_post_process` or `bevy::post_process`.
