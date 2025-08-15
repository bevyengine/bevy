---
title: "`Handle::Weak` has been replaced by `Handle::Uuid`."
pull_requests: [19896]
---

`Handle::Weak` had some weird behavior. It allowed for a sprite to be given a handle that is dropped
**while the sprite is still using it**. This also resulted in more complexity in the asset system.
The primary remaining use for `Handle::Weak` is to store asset UUIDs initialized through the
`weak_handle!` macro. To address this, `Handle::Weak` has been replaced by `Handle::Uuid`!

Users using the `weak_handle!` macro should switch to the `uuid_handle!` macro.

```rust
// Before
const IMAGE: Handle<Image> = weak_handle!("b20988e9-b1b9-4176-b5f3-a6fa73aa617f");

// After
const IMAGE: Handle<Image> = uuid_handle!("b20988e9-b1b9-4176-b5f3-a6fa73aa617f");
```

Users using `Handle::clone_weak` can (most likely) just call `Handle::clone` instead.

```rust
// Somewhere in some startup system.
let my_sprite_image = asset_server.load("monster.png");

// In game code...
// This sprite could be unloaded even if the sprite is still using it!
commands.spawn(Sprite::from_image(my_sprite_image.clone_weak()));

// Just do this instead!
commands.spawn(Sprite::from_image(my_sprite_image.clone()));
```

Users using the `Handle::Weak` variant directly should consider replacing it with `AssetId` instead,
accessible through `Handle::id`. These situations are very case-by-case migrations.

P.S., for users of the `weak_handle!` macro: If you are using it for shaders, consider switching to
`load_shader_library`/`load_embedded_asset` instead (especially replacing `load_internal_asset`).
This enables hot reloading for your shaders - which Bevy internally has done this cycle!
