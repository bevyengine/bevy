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

/// A map storing all application preferences.
///
/// Preferences are strongly typed, and defined independently by any `Plugin` that needs persistent
/// settings. Choice of serialization format and behavior is up to the application developer. The
/// preferences resource simply provides a common API surface to consolidate preferences for all
/// plugins in one location.
///
/// ### Usage
///
/// Preferences only require that a type being added implements [`Reflect`].
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
///
/// ### Serialization
///
/// The preferences map is build on `bevy_reflect`. This makes it possible to serialize preferences
/// into a dynamic structure, and deserialize it back into this map, while retaining a
/// strongly-typed API. Because it uses `serde`, `Preferences` can be read ad written to any format.
///
/// To build a storage backend, use [`Self::iter_reflect`] to get an iterator of `reflect`able trait
/// objects that can be serialized. To load serialized data into the preferences, use
/// `ReflectDeserializer` on each object to convert them into `Box<dyn Reflect>` trait objects,
/// which you can then load into this resource using [`Preferences::set_dyn`].
#[derive(Resource, Default, Debug)]
pub struct Preferences {
    // Note the key is only used while the struct is in memory so we can quickly look up a value.
    // The key itself does not need to be dynamic. This `DynamicMap` could be replaced with a custom
    // built data structure to (potentially) improve lookup performance, however it functions
    // perfectly fine for now.
    map: bevy_reflect::DynamicMap,
}

impl Preferences {
    /// Set preferences entry of type `P`, potentially overwriting an existing entry.
    pub fn set<P: Reflect>(&mut self, value: P) {
        let path = value.reflect_short_type_path().to_string();
        self.map.insert(path, value);
    }

    /// Set preferences entry from a boxed trait object of unknown type.
    pub fn set_dyn(&mut self, value: Box<dyn Reflect>) {
        let path = value.reflect_short_type_path().to_string();
        self.map.insert_boxed(Box::new(path), value);
    }

    /// Get preferences entry of type `P`.
    pub fn get<P: Reflect + TypePath>(&self) -> Option<&P> {
        let key = P::short_type_path().to_string();
        self.map
            .get(key.as_reflect())
            .and_then(|val| val.downcast_ref())
    }

    /// Get a mutable reference to a preferences entry of type `P`.
    pub fn get_mut<P: Reflect + TypePath>(&mut self) -> Option<&mut P> {
        let key = P::short_type_path().to_string();
        self.map
            .get_mut(key.as_reflect())
            .and_then(|val| val.downcast_mut())
    }

    /// Iterator over all preference entries as [`Reflect`] trait objects.
    pub fn iter_reflect(&self) -> impl Iterator<Item = &dyn Reflect> {
        self.map.iter().map(|(_k, v)| v)
    }

    /// Remove and return an entry from preferences, if it exists.
    pub fn remove<P: Reflect + TypePath>(&mut self) -> Option<Box<P>> {
        let key = P::short_type_path().to_string();
        self.map
            .remove(key.as_reflect())
            .and_then(|val| val.downcast().ok())
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::ResMut;
    use bevy_reflect::{Map, Reflect};
    use serde_json::Value;

    use crate::{App, PreferencesPlugin, Startup};

    use super::Preferences;

    #[derive(Reflect, PartialEq, Debug)]
    struct Foo(usize);

    #[derive(Reflect, PartialEq, Debug)]
    struct Bar(String);

    fn get_registry() -> bevy_reflect::TypeRegistry {
        let mut registry = bevy_reflect::TypeRegistry::default();
        registry.register::<Foo>();
        registry.register::<Bar>();
        registry
    }

    #[test]
    fn setters_and_getters() {
        let mut preferences = Preferences::default();

        // Set initial value
        preferences.set(Foo(36));
        assert_eq!(preferences.get::<Foo>().unwrap().0, 36);

        // Overwrite with set
        preferences.set(Foo(500));
        assert_eq!(preferences.get::<Foo>().unwrap().0, 500);

        // Overwrite with get_mut
        *preferences.get_mut().unwrap() = Foo(12);
        assert_eq!(preferences.get::<Foo>().unwrap().0, 12);

        // Add new type of preference
        assert!(preferences.get::<Bar>().is_none());
        preferences.set(Bar("Bevy".into()));
        assert_eq!(preferences.get::<Bar>().unwrap().0, "Bevy");

        // Add trait object
        preferences.set_dyn(Box::new(Bar("Boovy".into())));
        assert_eq!(preferences.get::<Bar>().unwrap().0, "Boovy");

        // Remove a preference
        assert_eq!(*preferences.remove::<Foo>().unwrap(), Foo(12));
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

    #[test]
    fn serialization_round_trip() {
        use bevy_reflect::serde::ReflectDeserializer;
        use serde::{de::DeserializeSeed, Serialize};

        let mut preferences = Preferences::default();
        preferences.set(Foo(42));
        preferences.set(Bar("Bevy".into()));

        let mut output = String::new();
        output.push('[');
        let registry = get_registry();

        for value in preferences.iter_reflect() {
            let serializer = bevy_reflect::serde::ReflectSerializer::new(value, &registry);
            let mut buf = Vec::new();
            let format = serde_json::ser::PrettyFormatter::with_indent(b"    ");
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, format);
            serializer.serialize(&mut ser).unwrap();

            let value_output = std::str::from_utf8(&buf).unwrap();
            output.push_str(value_output);
            output.push(',');
        }
        output.pop();
        output.push(']');

        let expected = r#"[{
    "bevy_app::preferences::tests::Foo": [
        42
    ]
},{
    "bevy_app::preferences::tests::Bar": [
        "Bevy"
    ]
}]"#;
        assert_eq!(expected, output);

        // Reset preferences and attempt to round-trip the data.

        let mut preferences = Preferences::default();
        assert!(preferences.map.is_empty());

        let json: Value = serde_json::from_str(&output).unwrap();
        let entries = json.as_array().unwrap();

        for entry in entries {
            // Convert back to a string and re-deserialize. Is there an easier way?
            let entry = entry.to_string();
            let mut deserializer = serde_json::Deserializer::from_str(&entry);

            let reflect_deserializer = ReflectDeserializer::new(&registry);
            let output: Box<dyn Reflect> =
                reflect_deserializer.deserialize(&mut deserializer).unwrap();
            let type_id = output.get_represented_type_info().unwrap().type_id();
            let reflect_from_reflect = registry
                .get_type_data::<bevy_reflect::ReflectFromReflect>(type_id)
                .unwrap();
            let value: Box<dyn Reflect> = reflect_from_reflect.from_reflect(&*output).unwrap();
            dbg!(&value);
            preferences.set_dyn(value);
        }

        assert_eq!(preferences.get(), Some(&Foo(42)));
        assert_eq!(preferences.get(), Some(&Bar("Bevy".into())));
    }
}
