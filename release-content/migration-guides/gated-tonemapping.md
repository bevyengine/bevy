---
title: `Tonemapping` modes `ToneMcMapface`, `BlenderFilmic`, and `AgX` are now gated behind `tonemapping_luts`
pull_requests: [20924]
---

`Tonemapping` mode `ToneMcMapface`, `BlenderFilmic`, and `AgX` are now only present with the `tonemapping_luts`
instead of having a notice on the documentation and logging an error during runtime.
