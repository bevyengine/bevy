---
title: bevy-settings errors on useless `SettingsGroup`s
pull_requests: [25548]
---

The `SettingsGroup` trait now has trait bounds `Resource + Reflect + Default`, as opposed to (originally) `Resource`. This is because types that implement `SettingsGroup` will not gain any of the benefits of `SettingsGroup` (ie automatic saving and loading caused by the system included in the settings plugin) unless the type is `Reflect + Default`.

To migrate code:
- Implement the `Reflect` and `Default` traits on your settings group type.
- Annotate your settings group type with `#[reflect(Default, SettingsGroup)]`.

If your code was already working, and the type implementing `SettingsGroup` was already saving and loading, you should not need to change anything in your code. The changes were intended to avoid breaking any already functioning code. 
