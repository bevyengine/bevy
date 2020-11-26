use crate::serde::SceneDeserializer;
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_ecs::{FromResources, Resources};
use bevy_property::PropertyTypeRegistry;
use bevy_type_registry::TypeRegistry;
use bevy_utils::BoxedFuture;
use parking_lot::RwLock;
use serde::de::DeserializeSeed;
use std::sync::Arc;

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

impl AssetLoader for SceneLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let registry = self.property_type_registry.read();
            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            let scene_deserializer = SceneDeserializer {
                property_type_registry: &registry,
            };
            let scene = scene_deserializer.deserialize(&mut deserializer)?;
            load_context.set_default_asset(LoadedAsset::new(scene));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["scn"]
    }
}
