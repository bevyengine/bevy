---
title: "X11 is no longer enabled by default"
pull_requests: [21158]
---

X11 support is on the downturn, with other players like GTK announcing that
they're considering deprecating X11 support in the next major release and
Fedora 43 later this year is going to be Wayland only. With this in mind,
the `x11` top level feature on the Bevy crate is no longer a default
feature when building for Linux targets.

If your project was already targeting Wayland-only systems, this is
effectively a no-op and can safely be ignored.

If you still require X.Org support, you can manually reenable the `x11`
feature:

```toml
# 0.17
[dependencies]
bevy = { version = 0.17 }

# 0.17
[dependencies]
bevy = { version = 0.17, features = ["x11"] }
```
