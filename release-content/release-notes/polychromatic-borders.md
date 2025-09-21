---
title: Per-side UI border colors
authors: ["@robtfm"]
pull_requests: [18682]
---

TODO: add image from 18682

`bevy_ui` now supports distinct border colors on each side of your UI nodes,
controlled with the [`BorderColor`] component.
This feature was borrowed from CSS, where it is commonly used to fake buttons with depth,
but we're looking forward to seeing your creative designs.

[`BorderColor`]: https://docs.rs/bevy/0.17.0-rc.1/bevy/prelude/struct.BorderColor.html
