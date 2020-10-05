use crate::{serde::SceneDeserializer, Scene};
use anyhow::Result;
use bevy_asset::AssetLoader;
use bevy_ecs::{FromResources, Resources};
use bevy_property::PropertyTypeRegistry;
use bevy_type_registry::TypeRegistry;
use parking_lot::RwLock;
use serde::de::DeserializeSeed;
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub struct SceneLoader {
    property_type_registry: Arc<RwLock<PropertyTypeRegistry>>,
}

impl FromResources for SceneLoader {
    fn from_resources(resources: &Resources) -> Self {
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        SceneLoader {
            property_type_registry: type_registry.property.clone(),
        }
    }
}

impl AssetLoader<Scene> for SceneLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Scene> {
        let registry = self.property_type_registry.read();
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let scene_deserializer = SceneDeserializer {
            property_type_registry: &registry,
        };
        let scene = scene_deserializer.deserialize(&mut deserializer)?;
        Ok(scene)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["scn"];
        EXTENSIONS
    }
}
