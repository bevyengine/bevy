---
title: "Per-RenderPhase Draw Functions"
pull_requests: [21021]
---

This PR makes draw function labels in `MaterialProperties` per-`RenderPhase` instead
of per-pass. This should only affect users of the low-level "manual Material" API,
and not users of the broader Material API. Specifying all draw functions is not
mandatory, but users should specify draw functions for all render phases the
material may queue to, or the material may not render.

- Removed `MaterialDrawFunction` in favor of:
  - `MainPassOpaqueDrawFunction`
  - `MainPassAlphaMaskDrawFunction`
  - `MainPassTransmissiveDrawFunction`
  - `MainPassTransparentDrawFunction`

- Removed `PrepassDrawFunction` in favor of:
  - `PrepassOpaqueDrawFunction`
  - `PrepassAlphaMaskDrawFunction`

- Removed `DeferredDrawFunction` in favor of:
  - `DeferredOpaqueDrawFunction`
  - `DeferredAlphaMaskDrawFunction`
