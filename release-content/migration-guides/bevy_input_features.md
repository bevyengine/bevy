---
title: Put input sources for `bevy_input` under features
pull_requests: [21447]
---

`bevy_input` provides primitives for all kinds of input. But on
consoles you usually don't have things like touch. On more obscure
platforms, like GBA, only gamepad input is needed.

If you use `bevy_window` or `bevy_gilrs`, they will automatically
enable the necessary features on `bevy_input`. If you don't depend
on them (for example, if you are developing for a platform that
isn't supported by these crates), you need to enable the required
input sources on the `bevy_input` / `bevy` crate manually:

```toml
# 0.17
bevy = { version = "0.17", default-features = false }

# 0.18 (enable sources that you actually use):
bevy = { version = "0.18", default-features = false, features = [
  "mouse",
  "keyboard",
  "gamepad",
  "touch",
  "gestures",
] }
```
