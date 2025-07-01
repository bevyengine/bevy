---
title: `Handle::Weak` has been replaced by `Handle::Uuid`.
pull_requests: [19896]
---

`Handle::Weak` has some weird behavior. It allows for a sprite to be given a handle that is dropped
**while the sprite is still using it**. This also results in more complexity in the asset system.
The primary remaining use for `Handle::Weak` is to store asset UUIDs initialized through the
`weak_handle!` macro. To address this, `Handle::Weak` has been replaced by `Handle::Uuid`!

Users using the `weak_handle!` macro should switch to the `uuid_handle!` macro.

```rust
# Before
const IMAGE: Handle<Image> = weak_handle!("12345678-9abc-def0-1234-56789abcdef0");

# After
const IMAGE: Handle<Image> = uuid_handle!("12345678-9abc-def0-1234-56789abcdef0");
```

Users using the `Handle::Weak` variant directly should consider replacing it with `AssetId` instead,
accessible through `Handle::id`. These situations are very case-by-case migrations.

P.S., for users of the `weak_handle!` macro: If you are using it for shaders, consider switching to
`load_shader_library`/`load_embedded_asset` instead (especially replacing `load_internal_asset`).
This enables hot reloading for your shaders - which Bevy internally has done this cycle!
