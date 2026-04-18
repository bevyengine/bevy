---
title: "Remove android game activity from default"
pull_requests: [23708]
---

Bevy previously had `"android-game-activity"` as part of its default features. Users that wanted to use "android-native-activity" instead, had to disable `default-features` and define all features + "android-native-activity" explicitly since they can't have both activity features at once.

Now both activities are not part of the default features but need to be added explicitly.

**Migration from bevy 0.18 to 0.19:**

For apps using `GameActivity` you need to add the feature `"android-game-activity"` to your `Cargo.toml`:

```toml
bevy = { version = "0.19", features = ["android-game-activity"] }
```

Since apps using `NativeActivity` already define features explicitly, you don't have to necessarily make changes. If you want to use the default features instead, you can now just add the feature `"android-native-activity"` to your `Cargo.toml` instead of redefining features explicitly:

```toml
bevy = { version = "0.19", features = ["android-native-activity"] }
```
