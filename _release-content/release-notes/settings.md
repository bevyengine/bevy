---
title: "User settings"
authors: ["@viridia", "@mpowell90"]
pull_requests: [22891, 23034, 23719, 23812]
---

The Bevy editor needs a settings system — for layout preferences, tool configuration, and everything else that should persist between sessions. We've built `bevy_settings` as a proper standalone crate so that both the editor and your own games can share a solid, easy-to-use foundation.

You might want to persist:

- Editor panel layouts and tool preferences
- Music and sound volume controls
- Graphics options
- Window position and size
- "Don't show this dialog again"

## Defining settings

Settings groups are plain Rust structs that derive `Resource`, `SettingsGroup`, and `Reflect`:

```rust
#[derive(Resource, SettingsGroup, Reflect, Default)]
#[reflect(Resource, SettingsGroup, Default)]
struct AudioSettings {
    music_volume: f32,
    sfx_volume: f32,
}
```

Adding `PreferencesPlugin` with a unique [reverse-domain] app name will automatically load your settings groups
on startup and insert them as resources:

```rust
app.add_plugins(PreferencesPlugin::new("com.example.mygame"));
```

Once the settings groups are added, you can read them like any other resource:

```rust
fn adjust_volume(audio: Res<AudioSettings>, mut music: ResMut<AudioSink>) {
    music.set_volume(audio.music_volume);
}
```

[reverse-domain]: https://en.wikipedia.org/wiki/Reverse_domain_name_notation

## Saving

To save after a change, queue a `SavePreferencesDeferred` command with a short debounce delay
so rapid changes don't hammer the filesystem:

```rust
fn save_settings_on_volume_changed(
    settings: Res<AudioSettings>,
    mut commands: Commands,
) {
    if !settings.is_changed(){
        return;
    }

    commands.queue(SavePreferencesDeferred(Duration::from_secs_f32(0.5)));
}
```

For save-on-quit (e.g., when the window closes), use `SavePreferencesSync::IfChanged` instead,
which blocks until the write completes before the app exits.

See the [`examples/app/persisting_preferences`](https://github.com/bevyengine/bevy/blob/latest/examples/app/persisting_preferences.rs) example for a complete walkthrough.

## Where files are stored

Settings are saved as TOML files in a folder named after your app's provided name (conventionally a reverse domain name),
inside the OS-specific preferences directory:

- **Linux**: `$XDG_CONFIG_HOME/<app_name>/` (typically `~/.config/<app_name>/`), following the [XDG Base Directory specification](https://specifications.freedesktop.org/basedir-spec/latest/)
- **macOS**: `~/Library/Preferences/<app_name>/`
- **Windows**: `%LOCALAPPDATA%\<app_name>\`
- **WASM**: browser `localStorage` (no filesystem)
- **Other platforms**: preferences are not persisted (`preferences_dir()` returns `None`)

This directory handling comes from the new `dirs` module in `bevy_platform`, which provides
`preferences_dir()` and other standard OS directory locations in a cross-platform way.

---

A special thanks to Andhrimnir (@tecbeast42) for giving Bevy ownership of the `bevy_settings` crate name on `crates.io`.
