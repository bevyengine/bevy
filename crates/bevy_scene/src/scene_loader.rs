use crate::ron;
#[cfg(feature = "serialize")]
use crate::serde::SceneDeserializer;
use crate::DynamicScene;
use bevy_asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::{FromWorld, World};
use bevy_utils::BoxedFuture;
#[cfg(feature = "serialize")]
use serde::de::DeserializeSeed;
use thiserror::Error;

/// [`AssetLoader`] for loading serialized Bevy scene files as [`DynamicScene`].
#[derive(Debug)]
pub struct SceneLoader {
    type_registry: AppTypeRegistry,
}

impl FromWorld for SceneLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>();
        SceneLoader {
            type_registry: type_registry.clone(),
        }
    }
}

/// Possible errors that can be produced by [`SceneLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SceneLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Error while trying to read the scene file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

#[cfg(feature = "serialize")]
impl AssetLoader for SceneLoader {
    type Asset = DynamicScene;
    type Settings = ();
    type Error = SceneLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            let scene_deserializer = SceneDeserializer {
                type_registry: &self.type_registry.read(),
            };
            Ok(scene_deserializer
                .deserialize(&mut deserializer)
                .map_err(|e| deserializer.span_error(e))?)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["scn", "scn.ron"]
    }
}
