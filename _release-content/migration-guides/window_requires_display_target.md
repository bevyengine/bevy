---
title: "`Window` requires `DisplayTarget`, and `WindowPlugin` has a new field"
pull_requests: [25658]
---

`Window` now requires the new `DisplayTarget` component, which requests the
dynamic range or color space, and the luminance, of the display output. Every window entity gets one
through the required-component machinery. The default requests SDR sRGB with
no calibrated luminance, and nothing reads the component yet, so rendering is
unchanged.

Things you may notice:

- Queries such as `Query<&DisplayTarget, With<Window>>` match every window
  entity, and code that assumes an exact set of components on a window entity
  must account for the extra one.
- Window entities serialized with reflection now include `DisplayTarget`.

`WindowPlugin` has a new field, `primary_display_target`, which sets the
component on the primary window in the same way `primary_cursor_options` does.
Code that constructs `WindowPlugin` without `..default()` must set it.
