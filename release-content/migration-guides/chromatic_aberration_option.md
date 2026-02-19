---
title: ChromaticAberration LUT is now Option
pull_requests: [19408]
---

The `ChromaticAberration` component `color_lut` field use to be a regular `Handle<Image>`. Now, it
is an `Option<Handle<Image>>` which falls back to the default image when `None`. For users assigning
a custom LUT, just wrap the value in `Some`.
