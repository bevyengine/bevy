//! This crate provides basic preferences support for Bevy applications. The word "preferences"
//! in this context is used to mean user settings that are (1) set while running the app, (2) persistent
//! across restarts, and (3) implicitly saved. It is not meant to be a general config file
//! serialization mechanism.
//!
//! Preferences typically include things like:
//!
//! - Current editing "mode" or tool.
//! - Keyboard or game controller bindings.
//! - Music and sound effects volume settings.
//! - The location of the last saved game.
//! - The user's login name for a network game (but not password!)
//! - "Do not show this dialog again" checkbox settings.
//!
//! Preferences are _NOT_ the same thing as saved games, assets, or platform config files.
//!
//! ## Supported Features
//!
//! - Supports both Desktop and Web (WASM) platforms.
//! - Preferences are serialized to TOML format.
//! - You can store most serde-compatible data types as preference settings.
//! - Preferences are saved in standard OS locations. Config directories are created if they do
//!   not already exist. The settings directory name is configurable.
//! - File-corruption-resistant: the framework will save the settings to a temp file, close the file,
//!   and then use a filesystem operation to move the temporary file to the settings config. This means
//!   that if the game crashes while saving, the settings file won't be corrupted.
//! - Debouncing/throttling - often a user setting, such as an audio volume slider or window
//!   splitter bar, changes at high frequency when dragged. The library allows you to mark preferences
//!   as "changed", which will save out preferences after a delay of one second.
//! - Various configurable options for saving preferences:
//!   - Mark changed: you can explicitly mark the preferences as "changed", which will trigger a
//!     deferred save.
//!   - Explicit synchronous flush: you can issue a [`Command`] which immediately and synchronously
//!     writes out the settings file.
//!
//! ## Platform support
//!
//! When compiling for WASM targets, preferences are stored in browser `LocalStorage`.
//!
//! When compiling for desktop, preferences are stored in the standard OS locations
//! for user preferences.
//!
//! ## Usage
//!
//! ### Preferences Structure
//!
//! The `Preferences` object represents the container for preferences files. Within this container
//! you can create individual [`PreferencesFile`] objects, each one backed by a separate file such as
//! "prefs.toml" (on the web, each file is stored as a separate key item in `LocalStorage`).
//!
//! Each preferences file contains one or more `PreferenceGroups` which represents a section within
//! the file. Groups can also contain other groups.
//!
//! Finally, groups have individual properties which are accessed via `get` and `set` methods.
//!
//! In the examples below, the `app.toml` file would have a structure like this:
//!
//! ```toml
//! [window]
//! size = [
//!     800,
//!     600
//! ]
//! ```
//!
//! ### Initializing the preferences store and loading preferences
//!
//! Normally the `Preferences` object is initialized during app initialization. You create a new
//! `Preferences` object, passing it a unique string which identifies your application. This string
//! is used to ensure that your preferences don't overwrite those of other installed apps.
//!
//! The [reverse domain name](https://en.wikipedia.org/wiki/Reverse_domain_name_notation) convention
//! is an easy way to ensure global uniqueness:
//!
//! ```rust,ignore
//! // Configure preferences directory
//! let mut preferences = Preferences::new("com.mydomain.coolgame");
//! ```
//!
//! If you don't own a domain, then feel free to use the `bevy.org` domain combined with your
//! github username: `org.bevy.<myusername>.<mygame>`.
//!
//! In desktop targets, the app name is used to establish a preferences directory in the standard
//! OS location for preferences. In WASM targets, the app name is used as part of the key for
//! browser local storage.
//!
//! The preferences store will verify that the preferences directory exists, but won't load anything
//! yet. To actually load preferences, you'll need to load a `PreferencesFile`, which corresponds
//! to individual preference files in your config directory such as `app.toml`:
//!
//! ```rust,ignore
//! let app_prefs = preferences.get("app").unwrap();
//! if let Some(window_group) = app_prefs.get_group("window") {
//!     if let Some(window_size) = window_group.get::<UVec2>("size") {
//!         // Configure window size
//!     }
//! }
//! ```
//!
//! So for example on Mac, the above code would look for a file in the location
//! "$HOME/Library/Preferences/com.mydomain.coolgame/app.toml".
//!
//! In WASM, it would look for a local storage key named "com.mydomain.coolgame-app".
//!
//! The `Preferences` object is also an ECS Resource, so you can insert it into the game world. This
//! makes it easy for other parts of the game code to load their preference settings. For example,
//! startup systems can inject preferences like any other resource.
//!
//! ```rust,ignore
//! app.insert_resource(preferences);
//! ```
//!
//! ### Setting Preferences
//!
//! To add or modify preferences, you can use the `mut` versions of the preference methods:
//!
//! ```rust,ignore
//! let mut app_prefs = preferences.get_mut("app").unwrap();
//! let window_group = app_prefs.get_group_mut("window").unwrap();
//! window_group.set("size", UVec2::new(10, 10));
//! ```
//!
//! The `mut` methods do several things:
//!
//! - They automatically create new preferences files and groups if they don't already exist.
//! - They store the new property value.
//! - They will compare with the previous value, and mark the preference file as changed
//!   if the new value is different.
//!
//! ### Saving Preferences
//!
//! Setting the preference value only changes the preferences setting in memory, it does not automatically
//! save the changes to disk. To trigger a save, you can issue a `SavePreferences` command:
//!
//! ```rust,ignore
//! commands.queue(SavePreferences::IfChanged);
//! ```
//!
//! This will cause any preference files to be saved if they are marked as changed. It's up to you
//! to decide when to save preferences, but they should be saved before the app exits.
//!
//! To avoid causing frame delays, the `SavePreferences` command spawns a thread to perform the
//! filesystem operations. Alternatively, you can use `SavePreferencesSync` which does the same thing,
//! but on the main thread. Or you can just call `.save()` on the `PreferencesStore` object.
//!
//! ### Autosaving
//!
//! The `AutosavePrefsPlugin` implements a timer which can be used to save preferences. Once you
//! install this plugin, you can then start the timer by issuing a command:
//!
//! ```rust,ignore
//! commands.queue(StartAutosaveTimer);
//! ```
//!
//! This command sets the save timer to 1 second, which counts down and then saves any changed
//! preference files when the timer goes off. This is useful for settings that change at high
//! frequency (like dragging an audio volume slider), reducing the number of writes to disk.
//!
//! Changes to preferences may be lost if you kill the app before the timer goes off. For this
//! reason, it's a good idea to also intercept the app exit event and save preferences before
//! quitting; however, even this won't be enough in some cases. For example, hitting Command-Q
//! on Mac OS terminates the app immediately with no app exit event. In practice, this isn't
//! usually a problem since users rarely quit the app immediately after adjusting a preference.

mod autosave;

pub use autosave::{AutosavePrefsPlugin, StartAutosaveTimer};

#[cfg(not(target_arch = "wasm32"))]
mod dirs;

mod prefs;
mod prefs_file;

#[cfg(not(target_arch = "wasm32"))]
mod store_fs;

#[cfg(target_arch = "wasm32")]
mod store_wasm;

use bevy_ecs::{system::Command, world::World};
#[cfg(not(target_arch = "wasm32"))]
pub use store_fs::StoreFs;

#[cfg(target_arch = "wasm32")]
pub use store_wasm::StoreWasm;

pub use crate::prefs::Preferences;

pub use self::prefs_file::{
    PreferencesFile, PreferencesFileContent, PreferencesGroup, PreferencesGroupMut,
};

/// A Command which saves preferences to disk. This blocks the command queue until saving
/// is complete. This variant is preferred when the app is about to exit.
#[derive(Default, PartialEq)]
pub enum SavePreferencesSync {
    /// Save preferences only if they have changed (based on [`PreferencesChanged` resource]).
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferencesSync {
    fn apply(self, world: &mut World) {
        let mut prefs = world.get_resource_mut::<Preferences>().unwrap();
        prefs.save(self == SavePreferencesSync::Always);
    }
}

/// A Command which saves preferences to disk. Actual FS operations happen in another thread.
/// This variant is preserved when saving preferences in mid-game, to avoid stutter.
#[derive(Default, PartialEq)]
pub enum SavePreferences {
    /// Save preferences only if they have changed (based on [`PreferencesChanged` resource]).
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferences {
    fn apply(self, world: &mut World) {
        let mut prefs = world.get_resource_mut::<Preferences>().unwrap();
        prefs.save_async(self == SavePreferences::Always);
    }
}
