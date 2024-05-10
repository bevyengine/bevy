use bevy_ecs::system::Resource;
use bevy_reflect::{Map, Reflect, TypePath};

use crate::Plugin;

/// Adds application [`Preferences`] functionality.
pub struct PreferencesPlugin;

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut crate::App) {
        app.init_resource::<Preferences>();
    }
}

/// A map that stores all application preferences.
///
/// Preferences are strongly typed, and defined independently by any `Plugin` that needs persistent
/// settings. Choice of serialization format and behavior is up to the application developer. The
/// preferences resource simply provides a common API surface to consolidate preferences for all
/// plugins in one location.
///
/// ### Usage
///
/// Preferences only require that the type implements [`Reflect`].
///
/// ```
/// # use bevy_reflect::Reflect;
/// #[derive(Reflect)]
/// struct MyPluginPreferences {
///     do_things: bool,
///     fizz_buzz_count: usize
/// }
/// ```
/// You can [`Self::get`] or [`Self::set`] preferences by accessing this type as a [`Resource`]
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_app::Preferences;
/// # use bevy_reflect::Reflect;
/// #
/// # #[derive(Reflect)]
/// # struct MyPluginPreferences {
/// #     do_things: bool,
/// #     fizz_buzz_count: usize
/// # }
/// #
/// fn update(mut prefs: ResMut<Preferences>) {
///     let settings = MyPluginPreferences {
///         do_things: false,
///         fizz_buzz_count: 9000,
///     };
///     prefs.set(settings);
///
///     // Accessing preferences only requires the type:
///     let mut new_settings = prefs.get::<MyPluginPreferences>().unwrap();
///
///     // If you are updating an existing struct, all type information can be inferred:
///     new_settings = prefs.get().unwrap();
/// }
/// ```
#[derive(Resource, Default, Debug)]
pub struct Preferences {
    map: bevy_reflect::DynamicMap,
}

impl Preferences {
    /// Set preferences of type `P`.
    pub fn set<P: Reflect + TypePath>(&mut self, value: P) {
        self.map.insert(P::short_type_path(), value);
    }

    /// Get preferences of type `P`.
    pub fn get<P: Reflect + TypePath>(&self) -> Option<&P> {
        let key = P::short_type_path();
        self.map
            .get(key.as_reflect())
            .and_then(|val| val.downcast_ref())
    }

    /// Get a mutable reference to preferences of type `P`.
    pub fn get_mut<P: Reflect + TypePath>(&mut self) -> Option<&mut P> {
        let key = P::short_type_path();
        self.map
            .get_mut(key.as_reflect())
            .and_then(|val| val.downcast_mut())
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::ResMut;
    use bevy_reflect::{Map, Reflect};

    use crate::{App, PreferencesPlugin, Startup};

    use super::Preferences;

    #[test]
    fn typed_get() {
        #[derive(Reflect, Clone, PartialEq, Debug)]
        struct FooPrefsV1 {
            name: String,
        }

        #[derive(Reflect, Clone, PartialEq, Debug)]
        struct FooPrefsV2 {
            name: String,
            age: usize,
        }

        let mut preferences = Preferences::default();

        let v1 = FooPrefsV1 {
            name: "Bevy".into(),
        };

        let v2 = FooPrefsV2 {
            name: "Boovy".into(),
            age: 42,
        };

        preferences.set(v1.clone());
        preferences.set(v2.clone());
        assert_eq!(preferences.get::<FooPrefsV1>(), Some(&v1));
        assert_eq!(preferences.get::<FooPrefsV2>(), Some(&v2));
    }

    #[test]
    fn overwrite() {
        #[derive(Reflect, Clone, PartialEq, Debug)]
        struct FooPrefs(String);

        let mut preferences = Preferences::default();

        let bevy = FooPrefs("Bevy".into());
        let boovy = FooPrefs("Boovy".into());

        preferences.set(bevy.clone());
        preferences.set(boovy.clone());
        assert_eq!(preferences.get::<FooPrefs>(), Some(&boovy));
    }

    #[test]
    fn init_exists() {
        #[derive(Reflect, Clone, PartialEq, Debug)]
        struct FooPrefs(String);

        let mut app = App::new();
        app.add_plugins(PreferencesPlugin);
        app.update();
        assert!(app.world().resource::<Preferences>().map.is_empty());
    }

    #[test]
    fn startup_sets_value() {
        #[derive(Reflect, Clone, PartialEq, Debug)]
        struct FooPrefs(String);

        let mut app = App::new();
        app.add_plugins(PreferencesPlugin);
        app.add_systems(Startup, |mut prefs: ResMut<Preferences>| {
            prefs.set(FooPrefs("Initial".into()));
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<Preferences>()
                .get::<FooPrefs>()
                .unwrap()
                .0,
            "Initial"
        );
    }
}
