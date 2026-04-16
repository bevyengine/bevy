---
title: "Remove android game activity from default"
pull_requests: [23708]
---

In the default features of bevy the `"android-game-activity"` was set. This blocks simply adding the `"android-native-activity"` together with the default features. Only one android activity could be set in one build.

To make it possible to use the native activity with the defaults, the game activity is removed.

migration from bevy 0.18 to 0.19

For apps with GameActivity you need to add the feature `"android-game-activity"` in your `cargo.toml`. See updated examples/mobile.

```toml
bevy = { version = "0.19", features = ["android-game-activity"] }
```

For apps with NativeActivity you do not need to change anything, but if you like, you should use now the default and add only the feature `"android-native-activity"` in your `cargo.toml`.

```toml
bevy = { version = "0.19", features = ["android-native-activity"] }
```
