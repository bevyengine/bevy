//! Framework for saving and loading user preferences in Bevy applications.
use core::any::TypeId;
use std::collections::HashMap;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    change_detection::Tick,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::Command,
    world::World,
};
use bevy_log::warn;
use bevy_reflect::{
    prelude::ReflectDefault, serde::TypedReflectDeserializer, Reflect, ReflectDeserialize,
    ReflectSerialize, TypeInfo,
};

#[cfg(not(target_arch = "wasm32"))]
mod store_fs;

#[cfg(target_arch = "wasm32")]
mod store_wasm;

use serde::de::DeserializeSeed;
#[cfg(not(target_arch = "wasm32"))]
use store_fs::PreferencesStore;

#[cfg(target_arch = "wasm32")]
use store_wasm::PreferencesStore;

/// Plugin to orchestrate loading and saving of preferences.
pub struct PreferencesPlugin {
    /// The name of the application. This is used to uniquely identify the preferences directory
    /// so as not to confuse it with other applications' preferences. To ensure global uniqueness,
    /// it is recommended to use a reverse domain name, e.g. "com.example.myapp".
    pub app_name: String,
}

impl PreferencesPlugin {
    /// Construct a new `PreferencesPlugin` for the givn application name. To ensure global
    /// uniqueness and avoid overwriting settings for other apps, it is recommended to use a
    /// reverse domain name, e.g. "com.example.myapp".
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }
}

impl Plugin for PreferencesPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Annotation for a type which overrides which preferences file the type's contents will be
/// written to. By default, all preferences are written to a file named "settings".
#[derive(Debug, Clone, Reflect)]
pub struct PreferencesFile(pub &'static str);

/// Annotation for a type which causes the type's contents to be placed in a named section
/// in the preferences file.
#[derive(Debug, Clone, Reflect)]
pub struct PreferencesGroup(pub &'static str);

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
    app_name: String,
    files: HashMap<&'static str, PreferenceFileManifest>,
}

/// A Command which saves preferences to disk. This blocks the command queue until saving
/// is complete.
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
        save_preferences(world, false, self == SavePreferencesSync::Always);
    }
}

fn resources_to_toml(
    world: &World,
    types: &bevy_reflect::TypeRegistry,
    manifest: &PreferenceFileManifest,
) -> toml::map::Map<String, toml::Value> {
    let mut table = toml::Table::new();
    for tid in manifest.resource_types.iter() {
        let ty = types.get(*tid).unwrap();
        let type_info = ty.type_info();
        let Some(cmp) = ty.data::<ReflectComponent>() else {
            continue;
        };
        let Some(ser) = ty.data::<ReflectSerialize>() else {
            continue;
        };

        if let TypeInfo::Struct(stinfo) = type_info
            && let Some(group) = stinfo
                .custom_attributes()
                .get::<PreferencesGroup>()
                .map(|g| g.0)
        {
            let Some(component_id) = world.components().get_id(*tid) else {
                continue;
            };

            // let Some(resource_tick) = world.get_resource_change_ticks_by_id(component_id) else {
            //     continue;
            // };

            let Some(res_entity) = world.resource_entities().get(component_id) else {
                continue;
            };
            let res_entity_ref = world.entity(*res_entity);

            let Some(reflect) = cmp.reflect(res_entity_ref) else {
                continue;
            };
            let ser_value = ser.get_serializable(reflect);

            let toml_value = toml::Value::try_from(&*ser_value).unwrap();
            table.insert(group.to_string(), toml_value);
        }
    }
    table
}

/// A Command which saves preferences to disk. Actual FS operations happen in another thread.
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
        save_preferences(world, true, self == SavePreferences::Always);
    }
}

fn save_preferences(world: &mut World, use_async: bool, _force: bool) {
    let this_run = world.change_tick();
    let Some(registry) = world.get_resource::<PreferencesFileRegistry>() else {
        warn!("Preferences registry not found - did you forget to call load_preferences()?");
        return;
    };
    let Some(app_types) = world.get_resource::<AppTypeRegistry>() else {
        return;
    };
    let app_types = app_types.clone();
    let types = app_types.read();

    for (filename, manifest) in registry.files.iter() {
        // TODO: See if changed unless _force is true
        // only save if file.last_save is >= the change time of all resources.
        let table = resources_to_toml(world, &types, manifest);
        let store = PreferencesStore::new(&registry.app_name);
        if use_async {
            store.save_async(filename, table);
        } else {
            store.save(filename, table);
        }
    }

    // Update timestamps
    let mut registry = world.get_resource_mut::<PreferencesFileRegistry>().unwrap();
    for (_, manifest) in registry.files.iter_mut() {
        manifest.last_save = this_run;
    }
}

/// Extension trait that implements loading of preferences into the application.
///
/// This needs to be called before `app.build()` so that preference values will be available
/// when the app is starting up.
pub trait LoadPreferences {
    /// Reads the preferences file and inserts or updates resources that are marked as preferences.
    fn load_preferences(&mut self) -> &mut Self;
}

impl LoadPreferences for App {
    fn load_preferences(&mut self) -> &mut Self {
        // Find the plugin so we can get the app name.
        let plugins = self.get_added_plugins::<PreferencesPlugin>();
        let Some(plugin) = plugins.first() else {
            warn!("Preference cannot be loaded; plugin not found.");
            return self;
        };
        let app_name = plugin.app_name.clone();
        let world = self.world();
        let last_save = world.read_change_tick();

        // Get the type registry and clone the Arc so we don't have to worry about borrowing.
        let Some(app_types) = world.get_resource::<AppTypeRegistry>() else {
            return self;
        };
        let app_types = app_types.clone();
        let types = app_types.read();

        // Build an index that remembers all of the resource types that are to be saved to
        // each individual settings file.
        let mut file_index = PreferencesFileRegistry {
            app_name: plugin.app_name.clone(),
            files: HashMap::new(),
        };

        // Scan through types looking for resources that have the neccessary traits and
        // annotations.
        for ty in types.iter() {
            if !(ty.contains::<ReflectResource>()
                && ty.contains::<ReflectSerialize>()
                && ty.contains::<ReflectDeserialize>()
                && ty.contains::<ReflectDefault>())
            {
                continue;
            };

            let type_info = ty.type_info();
            if let TypeInfo::Struct(stinfo) = type_info
                && let Some(_group) = stinfo
                    .custom_attributes()
                    .get::<PreferencesGroup>()
                    .map(|g| g.0)
            {
                // If no filename is specified, use "settings"
                let filename = stinfo
                    .custom_attributes()
                    .get::<PreferencesFile>()
                    .map_or("settings", |f| f.0);

                let pending_file =
                    file_index
                        .files
                        .entry(filename)
                        .or_insert(PreferenceFileManifest {
                            last_save,
                            resource_types: Vec::new(),
                        });
                pending_file.last_save = last_save;
                pending_file.resource_types.push(ty.type_id());
            }
        }

        // Now load each of the toml files we discovered, and apply their properties to
        // the resources in the world.
        let world = self.world_mut();
        let types = app_types.read();
        for (filename, manifest) in file_index.files.iter() {
            // Load the TOML file
            let store = PreferencesStore::new(&app_name);
            let toml = store.load(filename);
            if toml.is_none() {
                warn!("Filename {filename}.toml not found");
            }

            for tid in manifest.resource_types.iter() {
                let ty = types.get(*tid).unwrap();
                let type_info = ty.type_info();

                if let TypeInfo::Struct(stinfo) = type_info
                    && let Some(group) = stinfo
                        .custom_attributes()
                        .get::<PreferencesGroup>()
                        .map(|g| g.0)
                {
                    let reflect_component = ty.data::<ReflectComponent>().unwrap();
                    let component_id = world.components().get_id(*tid);
                    let res_entity =
                        component_id.and_then(|cid| world.resource_entities().get(cid));

                    let deserializer = TypedReflectDeserializer::new(ty, &types);
                    if let Some(res_entity) = res_entity {
                        // Resource already exists, so apply toml properties to it.
                        let res_entity_mut = world.entity_mut(*res_entity);
                        let Some(mut reflect) = reflect_component.reflect_mut(res_entity_mut)
                        else {
                            continue;
                        };

                        if let Some(ref toml) = toml
                            && let Some(value) = toml.get(group)
                        {
                            let new_value = deserializer.deserialize(value.clone()).unwrap();
                            reflect.apply(new_value.as_ref());
                        }
                    } else {
                        // The resource does not exist, so create a default.
                        let reflect_default = ty.data::<ReflectDefault>().unwrap();
                        let mut default_value = reflect_default.default();
                        let types = app_types.read();
                        let mut res_entity = world.spawn_empty();

                        if let Some(ref toml) = toml
                            && let Some(value) = toml.get(group)
                        {
                            let new_value = deserializer.deserialize(value.clone()).unwrap();
                            default_value.apply(new_value.as_ref());
                        }

                        reflect_component.insert(
                            &mut res_entity,
                            default_value.as_partial_reflect(),
                            &types,
                        );
                    }
                }
            }
        }

        drop(types);
        world.insert_resource::<PreferencesFileRegistry>(file_index);

        self
    }
}
