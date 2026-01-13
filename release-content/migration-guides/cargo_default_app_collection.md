---
title: `bevy_window`, `bevy_input_focus`, `custom_cursor` features moved to `common_api` collecition
pull_requests: [TODO: pr number here after creation]
---

In Bevy 0.18, [feature collections were introduced](https://bevy.org/learn/migration-guides/0-17-to-0-18/#cargo-feature-collections). In Bevy 0.19, the `bevy_window`, `bevy_input_focus`, & `custom_cursor` features were moved from the `default_app` collection to mid-level `common_api` collection.

This change was made because:
- the `default_app` collection is for core functionality that most apps will need. Scene definition for windowing is not usually required, and
- apps that don't use windowing (ex: command line tools, servers, etc) can compile fewer dependencies.

If you were relying on these being included in `default_app`, you can cherry-pick them into your `Cargo.toml` feature list:
```toml
# 0.18
bevy = { version = "0.18", default-features = false, features = [ "default_app" ] }

# 0.19
bevy = { version = "0.19", default-features = false, features = [
    "default_app",
    "bevy_window",
    "bevy_input_focus",
    "custom_cursor"
] }
```

If you already depend on a high-level profile (`2d`, `3d`, `ui`), or a mid-level collection ending in '`_render`' or '`_api`', then you do not need to make any changes.

---
I've erred on the side of hopefully-longer-than-it-needs-to-be.

