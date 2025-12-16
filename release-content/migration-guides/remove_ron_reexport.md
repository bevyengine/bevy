---
title: Remove ron re-export from bevy_scene and bevy_asset
pull_requests: [21611]
---

The `ron` crate is no longer re-exported from `bevy_scene` or `bevy_asset`. This was done to reduce naming conflicts and improve API clarity.

If you were importing `ron` through `bevy_scene` or `bevy_asset`, you should now add `ron` as a direct dependency to your project.

This change only affects code that was explicitly importing the `ron` module. All internal scene serialization and deserialization functionality remains unchanged.
