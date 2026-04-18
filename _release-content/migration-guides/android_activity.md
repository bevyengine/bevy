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

For apps with NativeActivity you do not need to change anything, but if you like, you should use now the default and add only the feature `"android-native-activity"` in your `cargo.toml`.

```toml
bevy = { version = "0.19", features = ["android-native-activity"] }
```
