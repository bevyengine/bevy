---
title: Physics Types
authors: ["@LizeLive"]
pull_requests: [21912]
---

Add a crate that provides physics component types for Bevy ECS, enabling rigid body
physics simulation representation.

## Why

To avoid making a new standard for describing physics.

## USD Physics Compatibility

Implements a 1:1 mapping with the [USD Physics Schema](https://openusd.org/release/api/usd_physics_page_front.html).
All components and attributes correspond directly to their USD Physics counterparts,
enabling seamless interchange with USD-based pipelines and tools.

### Limitations

Does not implement usd io. Only data standard.

### Bevy Adaptations

The following changes were made to align with Bevy conventions:

| USD Physics | Bevy | Reason |
| ------------- | ------ | -------- |
| Angles in degrees | Angles in radians | Bevy and most game engines use radians |
| `prim` terminology | `entity` terminology | Bevy ECS uses entities |
| `stage` terminology | `scene` terminology | Bevy uses scenes |
| `attribute` terminology | `component` terminology | Bevy ECS uses components |
| Relationships via paths | Relationships via `Entity` | Bevy uses entity references |
