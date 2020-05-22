use crate::{ComponentRegistry, ComponentRegistryContext, SceneDeserializer};
use anyhow::Result;
use bevy_app::FromResources;
use bevy_asset::AssetLoader;
use bevy_property::DynamicProperties;
use legion::prelude::{Resources, World};
use serde::de::DeserializeSeed;
use serde::{Serialize, Deserialize};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

pub struct DynamicScene {
    pub entities: Vec<SceneEntity>,
}

#[derive(Serialize, Deserialize)]
pub struct SceneEntity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}


#[derive(Default)]
pub struct Scene {
    pub world: World,
}

pub struct SceneLoader {
    component_registry: Arc<RwLock<ComponentRegistry>>,
}

impl FromResources for SceneLoader {
    fn from_resources(resources: &Resources) -> Self {
        let component_registry = resources
            .get::<ComponentRegistryContext>()
            .expect("SceneLoader requires the ComponentRegistry resource.");
        SceneLoader {
            component_registry: component_registry.value.clone(),
        }
    }
}

impl AssetLoader<Scene> for SceneLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Scene> {
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes).unwrap();
        let mut scene = Scene::default();
        let scene_deserializer = SceneDeserializer {
            component_registry: &self.component_registry.read().unwrap(),
            scene: &mut scene,
        };

        scene_deserializer.deserialize(&mut deserializer).unwrap();

        Ok(scene)
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["scn"];
        EXTENSIONS
    }
}
