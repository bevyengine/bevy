use crate::serde::SceneDeserializer;
use anyhow::Result;
use bevy_app::AppTypeRegistry;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_ecs::world::{FromWorld, World};
use bevy_reflect::TypeRegistryArc;
use bevy_utils::BoxedFuture;

#[cfg(feature = "serialize")]
use serde::de::DeserializeSeed;

#[derive(Debug)]
pub struct SceneLoader {
    type_registry: TypeRegistryArc,
}

impl FromWorld for SceneLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>();
        SceneLoader {
            type_registry: type_registry.0.clone(),
        }
    }
}

#[cfg(feature = "serialize")]
impl AssetLoader for SceneLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut deserializer = ron::de::Deserializer::from_bytes(bytes)?;
            let scene_deserializer = SceneDeserializer {
                type_registry: &self.type_registry.read(),
            };
            let scene = scene_deserializer.deserialize(&mut deserializer)?;
            load_context.set_default_asset(LoadedAsset::new(scene));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["scn", "scn.ron"]
    }
}
