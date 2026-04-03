---
title: "`bevy_window`, `bevy_input_focus`, `custom_cursor` features moved to alternate feature collections"
pull_requests: [22488]
---

In Bevy 0.18, [feature collections were introduced](https://bevy.org/learn/migration-guides/0-17-to-0-18/#cargo-feature-collections). The `bevy_window`, `bevy_input_focus`, & `custom_cursor` features were included in the `default_app` collection.

In Bevy 0.19, these have been moved from `default_app`:

|Feature           |is included in... |
|:----------------:|:-----------------|
|`bevy_window`     |`common_api`      |
|`bevy_input_focus`|`ui_api`          |
|`custom_cursor`   |`default_platform`|

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
