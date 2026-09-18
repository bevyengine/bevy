---
title: Entity inspection tools
authors: ["@jbuehler23", "@alice-i-cecile"]
pull_requests: [25818, 25822, 25823, 25824]
---

`bevy_dev_tools::inspection` is gaining a backend for inspecting worlds, entities, components, and resources.

This note will be completed once the rest of the series lands.

- Added `app.info` to the Bevy Remote Protocol (#25822)
- Added `diagnostics.list` and `diagnostics.get` to the Bevy Remote Protocol (#25824)
- Added `WorldSummary` to `bevy_dev_tools` inspection (#25818)
- Added component inspection to `bevy_dev_tools` (#25823)
