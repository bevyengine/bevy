use bevy_asset::{ron, AssetLoader, AsyncReadExt};
use bevy_math::{Rect, UVec2, Vec2};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{TextureAtlasBuilder, TextureAtlasLayout};

#[derive(Default)]
pub struct TextureAtlasLoader;

#[derive(Debug, Error)]
pub enum TextureAtlasError {
    #[error("failed to load file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode utf8 bytes: {0}")]
    Utf8Decode(#[from] std::string::FromUtf8Error),
    #[error("a RON error occurred during loading")]
    RonSpannedError(#[from] ron::error::SpannedError),
    #[error("failed to parse asset path")]
    ParseAssetPathError(#[from] bevy_asset::ParseAssetPathError),
    #[error("failed to build texture atlas")]
    TextureAtlasBuilderError(#[from] crate::TextureAtlasBuilderError),
    #[error("failed to load texture {0}")]
    LoadTextureError(#[from] bevy_asset::LoadDirectError),
    #[error("the path {0} points to an asset that is not an image")]
    NotATextureError(String),
}

#[derive(Debug, Deserialize, Serialize)]
struct TextureAtlasSer {
    size: Vec2,
    textures: Vec<Rect>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TextureAtlasGridSer {
    tile_size: Vec2,
    columns: usize,
    rows: usize,
    padding: Option<Vec2>,
    offset: Option<Vec2>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TextureAtlasMultiImageSer {
    textures: Vec<String>,
    padding: Option<UVec2>,
    max_atlas_size: Option<Vec2>,
}

impl AssetLoader for TextureAtlasLoader {
    type Asset = TextureAtlasLayout;
    type Settings = ();
    type Error = TextureAtlasError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy_asset::io::Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut bevy_asset::LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        if let Some(ext) = load_context.asset_path().get_full_extension() {
            if ext == GRID_EXTENSION {
                return Box::pin(async move {
                    let mut bytes = Vec::new();
                    reader.read_to_end(&mut bytes).await?;
                    let atlas_ser: TextureAtlasGridSer =
                        ron::de::from_str(&String::from_utf8(bytes)?)?;
                    let result = TextureAtlasLayout::from_grid(
                        atlas_ser.tile_size,
                        atlas_ser.columns,
                        atlas_ser.rows,
                        atlas_ser.padding,
                        atlas_ser.offset,
                    );
                    Ok(result)
                });
            }
            if ext == COMPOSED_EXTENSION {
                return Box::pin(async move {
                    let mut bytes = Vec::new();
                    reader.read_to_end(&mut bytes).await?;
                    let atlas_ser: TextureAtlasMultiImageSer =
                        ron::de::from_str(&String::from_utf8(bytes)?)?;

                    let mut builder = TextureAtlasBuilder::default().auto_format_conversion(true);
                    if let Some(max_size) = atlas_ser.max_atlas_size {
                        builder = builder.max_size(max_size);
                    }
                    if let Some(padding) = atlas_ser.padding {
                        builder = builder.padding(padding);
                    }

                    let mut textures = vec![];
                    for texture_path in atlas_ser.textures.iter() {
                        let texture = load_context
                            .load_direct(texture_path)
                            .await?
                            .take()
                            .ok_or_else(|| {
                                TextureAtlasError::NotATextureError(texture_path.clone())
                            })?;

                        textures.push(texture);
                    }

                    for (t, path) in textures.iter().zip(atlas_ser.textures.iter()) {
                        let handle = load_context.load(path);
                        builder.add_texture(Some(handle.into()), t);
                    }

                    let (atlas, img) = builder.finish()?;
                    load_context.add_labeled_asset("composed_texture".into(), img);

                    Ok(atlas)
                });
            }
        }

        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let atlas_ser: TextureAtlasSer = ron::de::from_str(&String::from_utf8(bytes)?)?;

            let mut layout = TextureAtlasLayout::new_empty(atlas_ser.size);
            for texture in atlas_ser.textures {
                layout.add_texture(texture);
            }
            Ok(layout)
        })
    }

    fn extensions(&self) -> &[&str] {
        &[COMMON_EXTENSION, GRID_EXTENSION, COMPOSED_EXTENSION]
    }
}

const COMMON_EXTENSION: &str = "atlas.ron";
const GRID_EXTENSION: &str = "atlas.grid.ron";
const COMPOSED_EXTENSION: &str = "atlas.composed.ron";
