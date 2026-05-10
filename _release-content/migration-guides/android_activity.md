---
title: "Remove android game activity from default"
pull_requests: [23708]
---

Bevy previously had `android-game-activity` as part of its default features. Users that wanted to use `android-native-activity` instead, had to disable `default-features` and define all features plus `android-native-activity` explicitly.

Both options are no longer part of `default-features`, but they need to be added explicitly.

For apps using `GameActivity` you need to add the `android-game-activity` feature to your `Cargo.toml`:

```toml
bevy = { version = "0.19", features = ["android-game-activity"] }
```

For apps using `NativeActivity` you no longer have to define all features explicitly, you can simply use:

```toml
bevy = { version = "0.19", features = ["android-native-activity"] }
```
