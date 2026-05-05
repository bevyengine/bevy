//! Framework for saving and loading user settings files (e.g. user preferences) in Bevy
//! applications.
//!
//! For purposes of this crate, the term "preferences" and "settings" are defined as:
//! * **Preferences** are configuration files that store persistent choices made by the end user
//!   while the app is running. Examples are audio volume, window position, or "show the tutorial".
//!   A key distinction is that these configuration files are consumed and produced by the same app.
//! * **Settings** is a more general term, which also includes configuration files produced by a
//!   different application, such as a text editor or external settings app.
//!
//! Refer to [`PreferencesPlugin`] for detailed usage information.

// Required to make proc macros work in bevy itself.
extern crate self as bevy_settings;

use core::any::TypeId;
use core::time::Duration;
use std::collections::HashMap;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::Tick,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Command, Commands, Res, ResMut},
    world::World,
};
pub use bevy_ecs_macros::SettingsGroup;
use bevy_log::warn;
use bevy_reflect::{
    prelude::ReflectDefault,
    serde::{TypedReflectDeserializer, TypedReflectSerializer},
    FromReflect, FromType, PartialReflect, ReflectMut, TypeInfo, TypePath, TypeRegistration,
    TypeRegistry,
};

#[cfg(not(target_arch = "wasm32"))]
mod store_fs;

#[cfg(target_arch = "wasm32")]
mod store_wasm;

use bevy_time::{Time, Timer, TimerMode};
use serde::de::DeserializeSeed;
#[cfg(not(target_arch = "wasm32"))]
use store_fs::PreferencesStore;

#[cfg(target_arch = "wasm32")]
use store_wasm::PreferencesStore;

/// Plugin to orchestrate loading and saving of user preferences.
///
/// You are required to provide a unique application name, so that your preferences don't overwrite
/// those of other apps. To ensure global uniqueness, it is recommended to use a
/// [reverse domain name](https://en.wikipedia.org/wiki/Reverse_domain_name_notation),
/// e.g. "com.example.myapp". The plugin will create a directory with that name in the
/// appropriate filesystem location (depending on platform) for app preferences. For platforms
/// without filesystems, other storage mechanisms will be used.
///
/// If you are do not have a domain name and cannot
/// afford one, use a reverse domain based on the URL of your repo (GitHub, GitLab, Codeberg
/// and so on).
///
/// Adding this plugin causes an immediate load of preferences (from either the filesystem or
/// browser local storage, depending on platform).
///
/// When using this plugin, care must be taken to ensure that plugins execute in the proper order.
/// Loading preferences causes registered settings to be inserted into the world as bevy resources.
/// You cannot access these values before they are loaded, but you may want to use the loaded values
/// when configuring other plugins. For this reason, it's generally a good idea to initialize and
/// load preferences before other plugins. The preferences plugin does not depend on any other
/// plugins.
///
/// In many cases, you may want to introduce additional "glue" plugins that copy preference
/// properties after they are loaded. For example, the
/// [`WindowPlugin`](https://docs.rs/bevy/latest/bevy/prelude/struct.WindowPlugin.html) plugin knows
/// nothing about preferences, but if you want the window size and position to persist between runs
/// you can add an additional plugin which copies the window settings from the resource to the
/// actual window entity.
///
/// Saving of preferences is not automatic; the recommended practice is to issue a
/// [`SavePreferencesDeferred`] command after modifying a settings resource. This will wait for
/// a short interval and then spawn an i/o task to write out the changed settings file. You can
/// also issue a [`SavePreferencesSync::IfChanged`] command immediately before exiting the app.
/// Note that on some platforms, depending on how the user exits (such as invoking Command-Q on
/// ``MacOS``) there may be no opportunity to intercept the app exit event, so the most reliable
/// approach is to use both techniques: deferred save and save-on-exit.
///
/// Saving is crash-resistant: if the app crashes in the middle of a save, the preferences file
/// will not be corrupted (it writes to a temporary file first, then uses atomic operations to
/// replace the previous file).
pub struct PreferencesPlugin {
    /// The unique name of the application.
    pub app_name: String,
}

impl PreferencesPlugin {
    /// Construct a new `PreferencesPlugin` for the given application name.
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }
}

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        let app_name = self.app_name.clone();
        let world = app.world();
        let last_save = world.read_change_tick();

        // Get the type registry and clone the Arc so we don't have to worry about borrowing.
        let Some(app_types) = world.get_resource::<AppTypeRegistry>() else {
            return;
        };
        let app_types = app_types.clone();
        let types = app_types.read();

        let world = app.world_mut();
        let file_index = build_preferences_registry(&app_name, &types, last_save);

        // Now load each of the toml files we discovered, and apply their properties to
        // the resources in the world.
        for (filename, manifest) in file_index.files.iter() {
            load_settings_file(world, &app_name, filename, manifest, &types);
        }

        // Cache the index so that we don't have to do it again when saving (and also makes
        // saving more deterministic).
        drop(types);
        world.insert_resource::<PreferencesFileRegistry>(file_index);

        app.add_systems(PostUpdate, handle_delayed_save);
    }
}

/// Trait which identifies a type as corresponding to a section with a settings file.
///
/// You can override the name of the section with `settings_group(group = "<name>")`.
/// For enum `SettingGroup`s, you can also override the name of its key with `settings_group(key = "<name>")`
/// The name should be in ``snake_case`` to be consistent with TOML style.
/// If there is a collision between names (multiple resources have the same name) then
/// the resulting properties will be merged into a single section.
///
/// You can also control which file the type gets saved to via
/// `settings_group(file = "<filename>")`. This should be the base name of the file without the
/// extension. The default name is `settings`, which will cause the preferences to be written out
/// to `settings.toml` in the app's preferences directory.
pub trait SettingsGroup: Resource {
    /// The name of the logical section within the settings file.
    fn settings_group_name() -> &'static str;

    /// The key name within the settings file.
    /// For structs, this should be set to `None`; The struct’s field names will be used as keys.
    /// For enums, the `SettingsGroup` will use this key name within the settings file for its sole key-value pair.
    /// This is typically the same as the group name, but can be customized.
    fn settings_key_name() -> Option<&'static str>;

    /// The name of the configuration file that contains this settings group.
    // TODO: Eventually convert this into an enum which represents various configuration sources.
    fn settings_source() -> Option<&'static str>;
}

/// Reflected data from a [`SettingsGroup`].
#[derive(Clone)]
pub struct ReflectSettingsGroup {
    /// The name of the logical section within the settings file.
    settings_group_name: &'static str,
    /// The key name within the settings file. Should only be `Some` for enums.
    settings_key_name: Option<&'static str>,
    /// The name of the settings file, defaults to "settings".
    settings_source: Option<&'static str>,
}

impl<T: SettingsGroup + FromReflect + TypePath> FromType<T> for ReflectSettingsGroup {
    fn from_type() -> Self {
        ReflectSettingsGroup {
            settings_group_name: T::settings_group_name(),
            settings_key_name: T::settings_key_name(),
            settings_source: T::settings_source(),
        }
    }

    fn insert_dependencies(type_registration: &mut TypeRegistration) {
        type_registration.register_type_data::<ReflectResource, T>();
    }
}

/// List of resource types that will be associated with a specific preferences file.
/// Also tracks when that file was last written or read.
#[derive(Default)]
struct PreferenceFileManifest {
    last_save: Tick,
    resource_types: Vec<TypeId>,
}

/// Records the game tick when preferences were last loaded or saved. This is used to determine
/// which preferences files have changed and need to be saved. Also tracks which settings files
/// are associated with which resource types.
#[derive(Resource)]
struct PreferencesFileRegistry {
    /// App name (from plugin)
    app_name: String,

    /// List of known preferences files, determined by scanning reflection registry.
    files: HashMap<&'static str, PreferenceFileManifest>,

    /// Timer used for batched saving.
    save_timer: Timer,
}

/// A Command which saves preferences to disk. This blocks the command queue until saving
/// is complete.
#[derive(Default, PartialEq)]
pub enum SavePreferencesSync {
    /// Save preferences only if they have changed since the most recent load or save.
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferencesSync {
    type Out = ();

    fn apply(self, world: &mut World) {
        save_preferences(world, false, self == SavePreferencesSync::Always);
    }
}

/// A [`Command`] which saves preferences to disk. Actual file system operations happen in another thread.
#[derive(Default, PartialEq)]
pub enum SavePreferences {
    /// Save preferences only if they have changed since the most recent load or save.
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferences {
    type Out = ();

    fn apply(self, world: &mut World) {
        save_preferences(world, true, self == SavePreferences::Always);
    }
}

/// A Command which saves changed preferences after a delay. This is debounced: issuing this
/// command multiple times resets the delay timer each time. This is meant to be used for settings
/// which change at a high frequency, such as dragging a slider which controls the game's audio
/// volume. The default delay is 1.0 seconds.
pub struct SavePreferencesDeferred(pub Duration);

impl Default for SavePreferencesDeferred {
    fn default() -> Self {
        Self(Duration::from_secs(1))
    }
}

impl Command for SavePreferencesDeferred {
    type Out = ();

    fn apply(self, world: &mut World) {
        let Some(mut registry) = world.get_resource_mut::<PreferencesFileRegistry>() else {
            return;
        };

        registry.save_timer.set_duration(self.0);
        registry.save_timer.reset();
        registry.save_timer.unpause();
    }
}

fn save_preferences(world: &mut World, use_async: bool, force: bool) {
    let this_run = world.change_tick();
    let Some(registry) = world.get_resource::<PreferencesFileRegistry>() else {
        warn!("Preferences registry not found - did you forget to install the PreferencesPlugin?");
        return;
    };
    let Some(app_types) = world.get_resource::<AppTypeRegistry>() else {
        return;
    };
    let app_types = app_types.clone();
    let types = app_types.read();

    for (filename, manifest) in registry.files.iter() {
        if force || has_preferences_changed(world, manifest) {
            let table = resources_to_toml(world, &types, manifest);
            let store = PreferencesStore::new(&registry.app_name);
            if use_async {
                store.save_async(filename, table);
            } else {
                store.save(filename, table);
            }
        }
    }

    // Update timestamps
    let mut registry = world.get_resource_mut::<PreferencesFileRegistry>().unwrap();
    for (_, manifest) in registry.files.iter_mut() {
        manifest.last_save = this_run;
    }
}

fn has_preferences_changed(world: &World, manifest: &PreferenceFileManifest) -> bool {
    let this_run = world.read_change_tick();
    manifest.resource_types.iter().any(|r| {
        let Some(component_id) = world.components().get_id(*r) else {
            return false;
        };
        if let Some(resource_change) = world.get_resource_change_ticks_by_id(component_id) {
            return resource_change.is_changed(manifest.last_save, this_run);
        }
        false
    })
}

fn resources_to_toml(
    world: &World,
    types: &TypeRegistry,
    manifest: &PreferenceFileManifest,
) -> toml::map::Map<String, toml::Value> {
    let mut table = toml::Table::new();

    for tid in manifest.resource_types.iter() {
        let ty = types.get(*tid).unwrap();

        let Some(cmp) = ty.data::<ReflectComponent>() else {
            continue;
        };

        let Some(reflect_settings_group) = ty.data::<ReflectSettingsGroup>() else {
            continue;
        };

        let settings_group = reflect_settings_group.settings_group_name;
        let settings_key = reflect_settings_group.settings_key_name;

        let Some(component_id) = world.components().get_id(*tid) else {
            continue;
        };

        let Some(res_entity) = world.resource_entities().get(component_id) else {
            continue;
        };
        let res_entity_ref = world.entity(res_entity);
        let Some(reflect) = cmp.reflect(res_entity_ref) else {
            continue;
        };

        let serializer = TypedReflectSerializer::new(reflect.as_partial_reflect(), types);

        let toml_value = if let Some(settings_key) = settings_key {
            // convert toml value into a key value pair if settings_key is set. settings_key is only set for enums
            toml::Value::Table(toml::Table::from_iter([(
                settings_key.to_string(),
                toml::Value::try_from(serializer).unwrap(),
            )]))
        } else {
            // Otherwise, the whole struct is serialized into toml
            toml::Value::try_from(serializer).unwrap()
        };

        match (
            toml_value.as_table(),
            table
                .get_mut(settings_group)
                .and_then(|value| value.as_table_mut()),
        ) {
            (Some(from), Some(to)) => {
                // Merge the tables
                for (key, value) in from.iter() {
                    to.insert(key.clone(), value.clone());
                }
            }
            _ => {
                table.insert(settings_group.to_string(), toml_value);
            }
        };
    }

    table
}

/// Builds the preferences file registry by scanning the type registry for settings resources.
/// This is separated from loading to enable testing without file I/O.
///
/// Returns the [`PreferencesFileRegistry`] that tracks which resources are associated with
/// which settings files.
fn build_preferences_registry(
    app_name: &str,
    types: &TypeRegistry,
    last_save: Tick,
) -> PreferencesFileRegistry {
    // Build an index that remembers all of the resource types that are to be saved to
    // each individual settings file.
    let mut file_index = PreferencesFileRegistry {
        app_name: app_name.to_string(),
        files: HashMap::new(),
        save_timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
    };
    file_index.save_timer.pause(); // Ensure timer is initially paused

    // Scan through types looking for resources that have the necessary traits and
    // annotations.
    for ty in types.iter() {
        if !ty.contains::<ReflectDefault>() {
            continue;
        };

        let Some(reflect_group) = ty.data::<ReflectSettingsGroup>() else {
            continue;
        };

        // If no filename is specified, use "settings"
        let filename = reflect_group.settings_source.unwrap_or("settings");
        let pending_file = file_index
            .files
            .entry(filename)
            .or_insert(PreferenceFileManifest {
                last_save,
                resource_types: Vec::new(),
            });
        pending_file.last_save = last_save;
        pending_file.resource_types.push(ty.type_id());
    }

    file_index
}

/// Loads a single settings file and applies its values to the world's resources.
fn load_settings_file(
    world: &mut World,
    app_name: &str,
    filename: &str,
    manifest: &PreferenceFileManifest,
    types: &TypeRegistry,
) {
    // Load the TOML file
    let store = PreferencesStore::new(app_name);
    let toml = store.load(filename);
    if toml.is_none() {
        warn!("Filename {filename}.toml not found");
    }

    apply_settings_to_world(world, toml.as_ref(), manifest, types);
}

/// Applies settings from a TOML table to the world's resources.
/// This is separated from file loading to enable testing without filesystem access.
///
/// For each resource type in the manifest, this function either:
/// - Updates an existing resource with values from the TOML, or
/// - Creates a new resource with default values merged with TOML values
fn apply_settings_to_world(
    world: &mut World,
    toml: Option<&toml::Table>,
    manifest: &PreferenceFileManifest,
    types: &TypeRegistry,
) {
    for tid in manifest.resource_types.iter() {
        let ty = types.get(*tid).unwrap();
        let Some(reflect_settings_group) = ty.data::<ReflectSettingsGroup>() else {
            continue;
        };

        let settings_group = reflect_settings_group.settings_group_name;
        let settings_key = reflect_settings_group.settings_key_name;

        let reflect_component = ty.data::<ReflectComponent>().unwrap();
        let component_id = world.components().get_id(*tid);
        let res_entity = component_id.and_then(|cid| world.resource_entities().get(cid));

        if let Some(res_entity) = res_entity {
            // Resource already exists, so apply toml properties to it.
            let res_entity_mut = world.entity_mut(res_entity);
            let Some(mut reflect) = reflect_component.reflect_mut(res_entity_mut) else {
                continue;
            };

            if let Some(toml) = toml
                && let Some(value) = toml.get(settings_group)
            {
                let value = if let Some(settings_key) = settings_key {
                    // If there is a settings key, then we need to look one level deeper in the TOML
                    // to find the actual properties to apply to the resource.
                    value.get(settings_key).unwrap_or(value)
                } else {
                    // No settings key, so we can apply the whole section to the resource
                    value
                };

                load_properties(value, &mut *reflect, types);
            }
        } else {
            // The resource does not exist, so create a default.
            let reflect_default = ty.data::<ReflectDefault>().unwrap();
            let mut default_value = reflect_default.default();
            let mut res_entity = world.spawn_empty();

            if let Some(toml) = toml
                && let Some(value) = toml.get(settings_group)
            {
                let value = if let Some(settings_key) = settings_key {
                    // If there is a settings key, then we need to look one level deeper in the TOML
                    // to find the actual properties to apply to the resource.
                    value.get(settings_key).unwrap_or(value)
                } else {
                    // No settings key, so we can apply the whole section to the resource
                    value
                };

                load_properties(value, &mut *default_value, types);
            }

            // Now add the new resource to the world.
            reflect_component.insert(&mut res_entity, default_value.as_partial_reflect(), types);
        }
    }
}

fn load_properties(value: &toml::Value, resource: &mut dyn PartialReflect, types: &TypeRegistry) {
    let Some(tinfo) = resource.get_represented_type_info() else {
        return;
    };

    match tinfo {
        TypeInfo::Struct(stinfo) => {
            if let Some(table) = value.as_table()
                && let ReflectMut::Struct(st_reflect) = resource.reflect_mut()
            {
                // Deserialize matching field names, ignore ones that don't match.
                for (idx, field) in stinfo.field_names().iter().enumerate() {
                    if let Some(toml_field_value) = table.get(*field)
                        && let Some(field_info) = stinfo.field_at(idx)
                        && let Some(field_type) = types.get(field_info.type_id())
                    {
                        let deserializer = TypedReflectDeserializer::new(field_type, types);
                        if let Ok(field_value) = deserializer.deserialize(toml_field_value.clone())
                        {
                            // Should be safe to unwrap here since we know the field exists (above).
                            st_reflect.field_at_mut(idx).unwrap().apply(&*field_value);
                        }
                    }
                }
            }
        }
        TypeInfo::TupleStruct(tstinfo) => {
            if let ReflectMut::TupleStruct(tst_reflect) = resource.reflect_mut() {
                // tuple structs with length > 1 are always serialized as arrays
                if tst_reflect.field_len() > 1
                    && let Some(array) = value.as_array()
                {
                    for (idx, toml_field_value) in array.iter().enumerate() {
                        if let Some(field_info) = tstinfo.field_at(idx)
                            && let Some(field_type) = types.get(field_info.type_id())
                        {
                            let deserializer = TypedReflectDeserializer::new(field_type, types);
                            if let Ok(field_value) =
                                deserializer.deserialize(toml_field_value.clone())
                            {
                                // Should be safe to unwrap here since we know the field exists (above).
                                tst_reflect.field_mut(idx).unwrap().apply(&*field_value);
                            }
                        }
                    }
                } else if tst_reflect.field_len() == 1
                    && let Some(field_info) = tstinfo.field_at(0)
                    && let Some(field_type) = types.get(field_info.type_id())
                {
                    let deserializer = TypedReflectDeserializer::new(field_type, types);
                    if let Ok(field_value) = deserializer.deserialize(value.clone()) {
                        // Should be safe to unwrap here since we know the field exists (above).
                        tst_reflect.field_mut(0).unwrap().apply(&*field_value);
                    }
                }
            }
        }
        TypeInfo::Enum(einfo) => {
            if let ReflectMut::Enum(en_reflect) = resource.reflect_mut()
                && let Some(variant_type) = types.get(einfo.type_id())
            {
                let deserializer = TypedReflectDeserializer::new(variant_type, types);

                if let Ok(variant_value) = deserializer.deserialize(value.clone()) {
                    en_reflect.apply(&*variant_value);
                }
            }
        }
        _ => {}
    }
}

fn handle_delayed_save(
    mut preferences: ResMut<PreferencesFileRegistry>,
    time: Res<Time>,
    mut commands: Commands,
) {
    preferences.save_timer.tick(time.delta());
    if preferences.save_timer.just_finished() {
        commands.queue(SavePreferences::IfChanged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::change_detection::Tick;
    use bevy_reflect::Reflect;

    /// Test resource that uses default settings group name (derived from type name)
    #[derive(Resource, SettingsGroup, Reflect, Default)]
    #[reflect(Resource, SettingsGroup, Default)]
    struct CounterSettings {
        count: i32,
    }

    /// Test resource that shares the same settings group name as another resource
    #[derive(Resource, SettingsGroup, Reflect, Default)]
    #[reflect(Resource, SettingsGroup, Default)]
    #[settings_group(group = "counter_settings")]
    struct ExtraCounterSettings {
        enabled: bool,
    }

    #[derive(Resource, SettingsGroup, Reflect, Debug, Default, PartialEq)]
    #[reflect(Resource, SettingsGroup, Default)]
    #[settings_group(group = "counter_settings", key = "refresh_rate")]
    enum CounterRefreshRateSettings {
        #[default]
        Slow,
        Fast,
    }

    /// Test resource that uses a different settings file
    #[derive(Resource, SettingsGroup, Reflect, Default)]
    #[reflect(Resource, SettingsGroup, Default)]
    #[settings_group(file = "audio")]
    struct AudioSettings {
        volume: f32,
    }

    #[test]
    fn test_build_registry_single_struct_resource() {
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();

        let registry = build_preferences_registry("test_app", &types, Tick::new(0));

        assert_eq!(registry.app_name, "test_app");
        assert_eq!(registry.files.len(), 1);
        assert!(registry.files.contains_key("settings"));

        let manifest = registry.files.get("settings").unwrap();
        assert_eq!(manifest.resource_types.len(), 1);
    }

    #[test]
    fn test_build_registry_single_enum_resource() {
        let mut types = TypeRegistry::default();
        types.register::<CounterRefreshRateSettings>();

        let registry = build_preferences_registry("test_app", &types, Tick::new(0));

        assert_eq!(registry.app_name, "test_app");
        assert_eq!(registry.files.len(), 1);
        assert!(registry.files.contains_key("settings"));

        let manifest = registry.files.get("settings").unwrap();
        assert_eq!(manifest.resource_types.len(), 1);
    }

    #[test]
    fn test_build_registry_merged_groups() {
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();
        types.register::<ExtraCounterSettings>();

        let registry = build_preferences_registry("test_app", &types, Tick::new(0));

        // Both resources should be in the same file
        assert_eq!(registry.files.len(), 1);
        assert!(registry.files.contains_key("settings"));

        let manifest = registry.files.get("settings").unwrap();
        // Both resources should be tracked
        assert_eq!(manifest.resource_types.len(), 2);
    }

    #[test]
    fn test_build_registry_separate_files() {
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();
        types.register::<AudioSettings>();

        let registry = build_preferences_registry("test_app", &types, Tick::new(0));

        // Resources should be in different files
        assert_eq!(registry.files.len(), 2);
        assert!(registry.files.contains_key("settings"));
        assert!(registry.files.contains_key("audio"));

        let settings_manifest = registry.files.get("settings").unwrap();
        assert_eq!(settings_manifest.resource_types.len(), 1);

        let audio_manifest = registry.files.get("audio").unwrap();
        assert_eq!(audio_manifest.resource_types.len(), 1);
    }

    #[test]
    fn test_resources_to_toml_merges_same_group() {
        let mut world = World::new();
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();
        types.register::<ExtraCounterSettings>();
        types.register::<CounterRefreshRateSettings>();

        // Insert both resources
        world.insert_resource(CounterSettings { count: 42 });
        world.insert_resource(ExtraCounterSettings { enabled: true });
        world.insert_resource(CounterRefreshRateSettings::Fast);

        // Build a manifest with both resource types
        let manifest = PreferenceFileManifest {
            last_save: Tick::new(0),
            resource_types: vec![
                TypeId::of::<CounterSettings>(),
                TypeId::of::<ExtraCounterSettings>(),
                TypeId::of::<CounterRefreshRateSettings>(),
            ],
        };

        let table = resources_to_toml(&world, &types, &manifest);

        // Both resources should be merged into the same "counter_settings" section
        assert!(table.contains_key("counter_settings"));
        let counter_section = table.get("counter_settings").unwrap().as_table().unwrap();

        // Check that fields are present in the merged section
        assert_eq!(
            counter_section.get("count").unwrap().as_integer().unwrap(),
            42
        );
        assert!(counter_section.get("enabled").unwrap().as_bool().unwrap());
        assert_eq!(
            counter_section
                .get("refresh_rate")
                .unwrap()
                .as_str()
                .unwrap(),
            "Fast"
        );
    }

    #[test]
    fn test_round_trip_serialization() {
        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug, Default)]
        #[reflect(Resource, SettingsGroup, Default)]
        struct SingleFieldTupleStruct(u8);

        #[derive(Reflect, PartialEq, Debug, Default)]
        #[reflect(Default)]
        struct NestedStruct {
            a: u8,
            b: u16,
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug, Default)]
        #[reflect(Resource, SettingsGroup, Default)]
        struct MultiFieldTupleStruct(u8, NestedStruct);

        #[derive(Resource, SettingsGroup, Reflect, Default)]
        #[reflect(Resource, SettingsGroup, Default)]
        struct NewTypeSingleTupleStruct(SingleFieldTupleStruct);

        #[derive(Resource, SettingsGroup, Reflect, Default)]
        #[reflect(Resource, SettingsGroup, Default)]
        struct NewTypeMultiTupleStruct(SingleFieldTupleStruct, MultiFieldTupleStruct);

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug, Default)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumUnitVariant {
            #[default]
            A,
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumSingleTupleVariant {
            A(u8),
        }

        impl Default for EnumSingleTupleVariant {
            fn default() -> Self {
                EnumSingleTupleVariant::A(0)
            }
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumMultiTupleVariant {
            A(u16, u32),
        }

        impl Default for EnumMultiTupleVariant {
            fn default() -> Self {
                EnumMultiTupleVariant::A(0, 0)
            }
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumStructVariant {
            A { x: u8, y: u16 },
        }

        impl Default for EnumStructVariant {
            fn default() -> Self {
                EnumStructVariant::A { x: 0, y: 0 }
            }
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumSingleNewTypeVariant {
            A(SingleFieldTupleStruct),
        }

        impl Default for EnumSingleNewTypeVariant {
            fn default() -> Self {
                EnumSingleNewTypeVariant::A(SingleFieldTupleStruct(0))
            }
        }

        #[derive(Resource, SettingsGroup, Reflect, PartialEq, Debug)]
        #[reflect(Resource, SettingsGroup, Default)]
        enum EnumMultiNewTypeVariant {
            A(SingleFieldTupleStruct, MultiFieldTupleStruct),
        }

        impl Default for EnumMultiNewTypeVariant {
            fn default() -> Self {
                EnumMultiNewTypeVariant::A(
                    SingleFieldTupleStruct(0),
                    MultiFieldTupleStruct(0, NestedStruct { a: 0, b: 0 }),
                )
            }
        }

        let mut world = World::new();
        let mut types = TypeRegistry::default();

        types.register::<CounterSettings>();
        types.register::<ExtraCounterSettings>();
        types.register::<CounterRefreshRateSettings>();
        types.register::<SingleFieldTupleStruct>();
        types.register::<MultiFieldTupleStruct>();
        types.register::<NewTypeSingleTupleStruct>();
        types.register::<NewTypeMultiTupleStruct>();
        types.register::<EnumUnitVariant>();
        types.register::<EnumSingleTupleVariant>();
        types.register::<EnumMultiTupleVariant>();
        types.register::<EnumStructVariant>();
        types.register::<EnumSingleNewTypeVariant>();
        types.register::<EnumMultiNewTypeVariant>();

        // Insert resources with specific values
        world.insert_resource(CounterSettings { count: 123 });
        world.insert_resource(ExtraCounterSettings { enabled: false });
        world.insert_resource(CounterRefreshRateSettings::Fast);
        world.insert_resource(SingleFieldTupleStruct(1));
        world.insert_resource(MultiFieldTupleStruct(2, NestedStruct { a: 1, b: 2 }));
        world.insert_resource(NewTypeSingleTupleStruct(SingleFieldTupleStruct(1)));
        world.insert_resource(NewTypeMultiTupleStruct(
            SingleFieldTupleStruct(1),
            MultiFieldTupleStruct(2, NestedStruct { a: 1, b: 2 }),
        ));
        world.insert_resource(EnumUnitVariant::A);
        world.insert_resource(EnumSingleTupleVariant::A(1));
        world.insert_resource(EnumMultiTupleVariant::A(1, 2));
        world.insert_resource(EnumStructVariant::A { x: 1, y: 2 });
        world.insert_resource(EnumSingleNewTypeVariant::A(SingleFieldTupleStruct(1)));
        world.insert_resource(EnumMultiNewTypeVariant::A(
            SingleFieldTupleStruct(1),
            MultiFieldTupleStruct(2, NestedStruct { a: 1, b: 2 }),
        ));

        // Build a manifest with both resource types
        let manifest = PreferenceFileManifest {
            last_save: Tick::new(0),
            resource_types: vec![
                TypeId::of::<CounterSettings>(),
                TypeId::of::<ExtraCounterSettings>(),
                TypeId::of::<CounterRefreshRateSettings>(),
                TypeId::of::<SingleFieldTupleStruct>(),
                TypeId::of::<MultiFieldTupleStruct>(),
                TypeId::of::<NewTypeSingleTupleStruct>(),
                TypeId::of::<NewTypeMultiTupleStruct>(),
                TypeId::of::<EnumUnitVariant>(),
                TypeId::of::<EnumSingleTupleVariant>(),
                TypeId::of::<EnumMultiTupleVariant>(),
                TypeId::of::<EnumStructVariant>(),
                TypeId::of::<EnumSingleNewTypeVariant>(),
                TypeId::of::<EnumMultiNewTypeVariant>(),
            ],
        };

        // Serialize to TOML
        let table = resources_to_toml(&world, &types, &manifest);

        // Create a new world and apply the TOML
        let mut new_world = World::new();
        apply_settings_to_world(&mut new_world, Some(&table), &manifest, &types);

        // Verify resources were created with correct values
        let counter = new_world.get_resource::<CounterSettings>().unwrap();
        assert_eq!(counter.count, 123);

        let extra = new_world.get_resource::<ExtraCounterSettings>().unwrap();
        assert!(!extra.enabled);

        let refresh_rate = new_world
            .get_resource::<CounterRefreshRateSettings>()
            .unwrap();
        assert_eq!(*refresh_rate, CounterRefreshRateSettings::Fast);

        let single_field_tuple_struct = new_world.get_resource::<SingleFieldTupleStruct>().unwrap();
        assert_eq!(single_field_tuple_struct.0, 1);

        let multi_field_tuple_struct = new_world.get_resource::<MultiFieldTupleStruct>().unwrap();
        assert_eq!(multi_field_tuple_struct.0, 2);
        assert_eq!(multi_field_tuple_struct.1.a, 1);
        assert_eq!(multi_field_tuple_struct.1.b, 2);

        let new_type_single_tuple_struct = new_world
            .get_resource::<NewTypeSingleTupleStruct>()
            .unwrap();
        assert_eq!(new_type_single_tuple_struct.0 .0, 1);

        let new_type_multi_tuple_struct =
            new_world.get_resource::<NewTypeMultiTupleStruct>().unwrap();
        assert_eq!(new_type_multi_tuple_struct.0 .0, 1);
        assert_eq!(new_type_multi_tuple_struct.1 .0, 2);
        assert_eq!(new_type_multi_tuple_struct.1 .1.a, 1);
        assert_eq!(new_type_multi_tuple_struct.1 .1.b, 2);

        let enum_unit_variant = new_world.get_resource::<EnumUnitVariant>().unwrap();
        assert_eq!(*enum_unit_variant, EnumUnitVariant::A);

        let enum_single_tuple_variant = new_world.get_resource::<EnumSingleTupleVariant>().unwrap();
        assert_eq!(*enum_single_tuple_variant, EnumSingleTupleVariant::A(1));

        let enum_multi_tuple_variant = new_world.get_resource::<EnumMultiTupleVariant>().unwrap();
        assert_eq!(*enum_multi_tuple_variant, EnumMultiTupleVariant::A(1, 2));

        let enum_struct_variant = new_world.get_resource::<EnumStructVariant>().unwrap();
        assert_eq!(*enum_struct_variant, EnumStructVariant::A { x: 1, y: 2 });

        let enum_single_new_type_variant = new_world
            .get_resource::<EnumSingleNewTypeVariant>()
            .unwrap();
        assert_eq!(
            *enum_single_new_type_variant,
            EnumSingleNewTypeVariant::A(SingleFieldTupleStruct(1))
        );

        let enum_multi_new_type_variant =
            new_world.get_resource::<EnumMultiNewTypeVariant>().unwrap();
        assert_eq!(
            *enum_multi_new_type_variant,
            EnumMultiNewTypeVariant::A(
                SingleFieldTupleStruct(1),
                MultiFieldTupleStruct(2, NestedStruct { a: 1, b: 2 })
            )
        );
    }

    #[test]
    fn test_round_trip_with_existing_resources() {
        let mut world = World::new();
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();
        types.register::<CounterRefreshRateSettings>();

        // Insert resource with initial values
        world.insert_resource(CounterSettings { count: 100 });
        world.insert_resource(CounterRefreshRateSettings::Fast);

        let manifest = PreferenceFileManifest {
            last_save: Tick::new(0),
            resource_types: vec![
                TypeId::of::<CounterSettings>(),
                TypeId::of::<CounterRefreshRateSettings>(),
            ],
        };

        // Serialize
        let table = resources_to_toml(&world, &types, &manifest);

        // Modify the resource
        world.resource_mut::<CounterSettings>().count = 999;
        *world.resource_mut::<CounterRefreshRateSettings>() = CounterRefreshRateSettings::Slow;

        // Apply TOML (should restore the original value)
        apply_settings_to_world(&mut world, Some(&table), &manifest, &types);

        let counter = world.get_resource::<CounterSettings>().unwrap();
        assert_eq!(counter.count, 100);
        let refresh_rate = world.get_resource::<CounterRefreshRateSettings>().unwrap();
        assert_eq!(*refresh_rate, CounterRefreshRateSettings::Fast);
    }

    #[test]
    fn test_partial_toml_preserves_missing_fields() {
        let mut world = World::new();
        let mut types = TypeRegistry::default();
        types.register::<CounterSettings>();
        types.register::<ExtraCounterSettings>();
        types.register::<CounterRefreshRateSettings>();

        // Insert resources with specific values
        world.insert_resource(CounterSettings { count: 50 });
        world.insert_resource(ExtraCounterSettings { enabled: true });
        world.insert_resource(CounterRefreshRateSettings::Fast);

        // Create a TOML table that only contains one field from one resource
        let mut table = toml::Table::new();
        let mut counter_section = toml::Table::new();
        counter_section.insert("count".to_string(), toml::Value::Integer(999));
        table.insert(
            "counter_settings".to_string(),
            toml::Value::Table(counter_section),
        );
        // Note: "enabled" field is missing from the TOML

        let manifest = PreferenceFileManifest {
            last_save: Tick::new(0),
            resource_types: vec![
                TypeId::of::<CounterSettings>(),
                TypeId::of::<ExtraCounterSettings>(),
                TypeId::of::<CounterRefreshRateSettings>(),
            ],
        };

        // Apply the partial TOML
        apply_settings_to_world(&mut world, Some(&table), &manifest, &types);

        // Verify count was updated
        let counter = world.get_resource::<CounterSettings>().unwrap();
        assert_eq!(counter.count, 999);

        // Verify enabled was preserved (not overwritten with default false)
        let extra = world.get_resource::<ExtraCounterSettings>().unwrap();
        assert!(extra.enabled);

        // Verify refresh_rate was preserved
        let refresh_rate = world.get_resource::<CounterRefreshRateSettings>().unwrap();
        assert_eq!(*refresh_rate, CounterRefreshRateSettings::Fast);
    }
}
