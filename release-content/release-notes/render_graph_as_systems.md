---
title: Render Graph as Systems
authors: [ "@tychedelia" ]
pull_requests: [ 22144 ]
---

Bevy's `RenderGraph` architecture has been replaced with schedules. Render passes are now regular systems that run in
the `Core3d`, `Core2d`, or custom rendering schedules.

The render graph was originally designed when Bevy's ECS was less mature. As our APIs have evolved, `Schedule`
has become capable of expressing the core render graph pattern. This change lets rendering better leverage familiar Bevy
patterns, as well as opening the door to new optimizations and ECS-driven features in the future, including plans for
parallel command buffer recording.
