#[cfg(feature = "serialize")]
use crate::serde::SceneDeserializer;
use crate::DynamicScene;
use bevy_asset::{anyhow, io::Reader, AssetLoader, AsyncReadExt, LoadContext};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::{FromWorld, World};
use bevy_reflect::TypeRegistryArc;
use bevy_utils::BoxedFuture;
#[cfg(feature = "serialize")]
use serde::de::DeserializeSeed;

/// [`AssetLoader`] for loading serialized Bevy scene files as [`DynamicScene`].
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
    type Asset = DynamicScene;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            let scene_deserializer = SceneDeserializer {
                type_registry: &self.type_registry.read(),
            };
            scene_deserializer
                .deserialize(&mut deserializer)
                .map_err(|e| {
                    let span_error = deserializer.span_error(e);
                    anyhow::anyhow!(
                        "{} at {}:{}",
                        span_error.code,
                        load_context.path().to_string_lossy(),
                        span_error.position,
                    )
                })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["scn", "scn.ron"]
    }
}
