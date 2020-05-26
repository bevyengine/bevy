use crate::{serde::SceneDeserializer, Scene};
use anyhow::Result;
use bevy_app::FromResources;
use bevy_asset::AssetLoader;
use bevy_component_registry::PropertyTypeRegistryContext;
use legion::prelude::Resources;
use serde::de::DeserializeSeed;
use std::{cell::RefCell, path::Path, rc::Rc};

pub struct SceneLoader {
    property_type_registry: PropertyTypeRegistryContext,
}

impl FromResources for SceneLoader {
    fn from_resources(resources: &Resources) -> Self {
        let property_type_registry = resources.get::<PropertyTypeRegistryContext>().unwrap();
        SceneLoader {
            property_type_registry: property_type_registry.clone(),
        }
    }
}

impl AssetLoader<Scene> for SceneLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Scene> {
        let registry = self.property_type_registry.value.read().unwrap();
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes).unwrap();
        let current_type_name = Rc::new(RefCell::new(None));
        let scene_deserializer = SceneDeserializer {
            property_type_registry: &registry,
            current_type_name: current_type_name.clone(),
        };

        // this callback is executed whenever an explicit type name is encountered in a map
        let mut callback = |ident: &Option<&[u8]>| {
            let mut last_type_name = current_type_name.borrow_mut();
            *last_type_name = ident.map(|i| String::from_utf8(i.to_vec()).unwrap());
        };
        deserializer.set_callback(&mut callback);

        let scene = scene_deserializer.deserialize(&mut deserializer).unwrap();
        Ok(scene)
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["scn"];
        EXTENSIONS
    }
}