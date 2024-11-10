#[cfg(feature = "serialize")]
use crate::serde::SceneDeserializer;
use crate::{ron, DynamicScene};
use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use bevy_ecs::{
    reflect::AppTypeRegistry,
    world::{FromWorld, World},
};
use bevy_reflect::TypeRegistryArc;
use derive_more::derive::{Display, Error, From};
#[cfg(feature = "serialize")]
use serde::de::DeserializeSeed;

/// Asset loader for a Bevy dynamic scene (`.scn` / `.scn.ron`).
///
/// The loader handles assets serialized with [`DynamicScene::serialize`].
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

/// Possible errors that can be produced by [`SceneLoader`]
#[non_exhaustive]
#[derive(Debug, Error, Display, From)]
pub enum SceneLoaderError {
    /// An [IO Error](std::io::Error)
    #[display("Error while trying to read the scene file: {_0}")]
    Io(std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[display("Could not parse RON: {_0}")]
    RonSpannedError(ron::error::SpannedError),
}

#[cfg(feature = "serialize")]
impl AssetLoader for SceneLoader {
    type Asset = DynamicScene;
    type Settings = ();
    type Error = SceneLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &self.type_registry.read(),
        };
        Ok(scene_deserializer
            .deserialize(&mut deserializer)
            .map_err(|e| deserializer.span_error(e))?)
    }

    fn extensions(&self) -> &[&str] {
        &["scn", "scn.ron"]
    }
}
