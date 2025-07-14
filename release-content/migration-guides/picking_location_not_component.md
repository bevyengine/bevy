---
title: Location is not a Component anymore
pull_requests: [19306]
---

`bevy_picking::Location` was erroneously made a `Component`. It is no longer one, `bevy_picking::PointerLocation` wraps a `Location` and is the intended usage pattern.
