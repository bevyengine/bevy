use bevy_ecs::{
    reflect::AppTypeRegistry,
    world::{FromWorld, World},
};
use bevy_reflect::{TypePath, TypeRegistryArc};

#[cfg(feature = "serialize")]
use {
    crate::{serde::WorldDeserializer, DynamicWorld},
    bevy_asset::{io::Reader, AssetLoader, LoadContext},
    serde::de::DeserializeSeed,
    thiserror::Error,
};

/// Asset loader for a Bevy dynamic world (`.scn` / `.scn.ron`).
///
/// The loader handles assets serialized with [`DynamicWorld::serialize`].
#[derive(Debug, TypePath)]
pub struct WorldAssetLoader {
    #[cfg_attr(
        not(feature = "serialize"),
        expect(dead_code, reason = "only used with `serialize` feature")
    )]
    type_registry: TypeRegistryArc,
}

impl FromWorld for WorldAssetLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>();
        WorldAssetLoader {
            type_registry: type_registry.0.clone(),
        }
    }
}

/// Possible errors that can be produced by [`WorldAssetLoader`]
#[cfg(feature = "serialize")]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum WorldAssetLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Error while trying to read the world file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

#[cfg(feature = "serialize")]
impl AssetLoader for WorldAssetLoader {
    type Asset = DynamicWorld;
    type Settings = ();
    type Error = WorldAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let scene_deserializer = WorldDeserializer {
            type_registry: &self.type_registry.read(),
            load_from_path: load_context,
        };
        Ok(scene_deserializer
            .deserialize(&mut deserializer)
            .map_err(|e| deserializer.span_error(e))?)
    }

    fn extensions(&self) -> &[&str] {
        &["scn", "scn.ron"]
    }
}
