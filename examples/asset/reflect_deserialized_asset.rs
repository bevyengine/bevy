//! Let's imagine we want to make a system where we can take an image, pass it
//! through a series of operations (a pipeline), and get back a new transformed
//! image.
//!
//! We want to define the pipeline using an asset, and the operations should be
//! fully dynamic - users should be able to register their own operation types,
//! and the asset loader can deserialize them without any extra setup.

use core::fmt;
use std::{fmt::Debug, io};

use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, LoadContext, ReflectHandle, RenderAssetUsages},
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectDeserializerProcessor},
        TypeRegistration, TypeRegistry, TypeRegistryArc,
    },
};
use image::{ColorType, DynamicImage};
use serde::de::{self, DeserializeSeed, Deserializer, Visitor};
use thiserror::Error;

/// Applies an operation to an image in the pipeline.
#[reflect_trait]
trait ImageOperation: Debug + Send + Sync + Reflect {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>);
}

struct ImageOperationContext<'a> {
    current: DynamicImage,
    image_assets: &'a Assets<Image>,
}

/// Series of [`ImageOperation`]s which may be applied to an image.
#[derive(Debug, Asset, TypePath)]
struct ImagePipeline {
    /// All operations applied to the image in order.
    ops: Vec<Box<dyn ImageOperation>>,
}

impl ImagePipeline {
    fn apply(&self, image: DynamicImage, image_assets: &Assets<Image>) -> DynamicImage {
        let mut ctx = ImageOperationContext {
            current: image,
            image_assets,
        };
        for op in &self.ops {
            op.apply(&mut ctx);
        }
        ctx.current
    }
}

// Operation implementations

/// Overwrites the current image with another image asset.
#[derive(Debug, Clone, Reflect)]
#[reflect(ImageOperation)]
struct Load {
    // Since this is a `Handle`, when we deserialize this in the asset loader,
    // we will also start a load for the asset that this handle points to.
    image: Handle<Image>,
}

impl ImageOperation for Load {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>) {
        if let Some(image) = ctx.image_assets.get(&self.image) {
            ctx.current = image
                .clone()
                .try_into_dynamic()
                .expect("image should be in a supported format");
        }
    }
}

/// Inverts all pixels in the image.
#[derive(Debug, Clone, Reflect)]
#[reflect(ImageOperation)]
struct Invert;

impl ImageOperation for Invert {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>) {
        ctx.current.invert();
    }
}

/// Applies a Gaussian blur to the image.
#[derive(Debug, Clone, Reflect)]
#[reflect(ImageOperation)]
struct Blur {
    /// Blur intensity.
    sigma: f32,
}

impl ImageOperation for Blur {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>) {
        ctx.current = ctx.current.blur(self.sigma);
    }
}

// Deserialization logic

/// Deserializes an [`ImagePipeline`].
struct ImagePipelineDeserializer<'a, 'b> {
    /// App type registry to use for getting registration type data.
    type_registry: &'a TypeRegistry,
    /// Asset loader context to use for starting loads when encountering a
    /// [`Handle`].
    load_context: &'a mut LoadContext<'b>,
}

impl<'de> DeserializeSeed<'de> for ImagePipelineDeserializer<'_, '_> {
    type Value = ImagePipeline;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PipelineVisitor<'a, 'b> {
            type_registry: &'a TypeRegistry,
            load_context: &'a mut LoadContext<'b>,
        }

        impl<'de> Visitor<'de> for PipelineVisitor<'_, '_> {
            type Value = ImagePipeline;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a list of image operations")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut ops = Vec::new();
                while let Some(op) = seq.next_element_seed(OperationDeserializer {
                    type_registry: self.type_registry,
                    load_context: self.load_context,
                })? {
                    ops.push(op);
                }
                Ok(ImagePipeline { ops })
            }
        }

        struct OperationDeserializer<'a, 'b> {
            type_registry: &'a TypeRegistry,
            load_context: &'a mut LoadContext<'b>,
        }

        impl<'de> DeserializeSeed<'de> for OperationDeserializer<'_, '_> {
            type Value = Box<dyn ImageOperation>;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                // Here's where we make use of the `ReflectDeserializerProcessor`.

                let mut processor = HandleProcessor {
                    load_context: self.load_context,
                };

                // Reflection boilerplate to deserialize a value reflexively
                // and convert it into a `Box<dyn ImageOperation>`.
                let value = ReflectDeserializer::with_processor(self.type_registry, &mut processor)
                    .deserialize(deserializer)?;
                let type_info = value.get_represented_type_info().ok_or_else(|| {
                    de::Error::custom(format!("{value:?} does not represent any type"))
                })?;
                let type_id = type_info.type_id();
                let type_path = type_info.type_path();

                let reflect_from_reflect = self
                    .type_registry
                    .get_type_data::<ReflectFromReflect>(type_id)
                    .ok_or_else(|| {
                        de::Error::custom(format!(
                            "`{type_path}` cannot be constructed reflexively via `FromReflect`"
                        ))
                    })?;
                let value = reflect_from_reflect
                    .from_reflect(value.as_ref())
                    .expect("should be able to convert value into represented type");

                let reflect_op = self
                    .type_registry
                    .get_type_data::<ReflectImageOperation>(type_id)
                    .ok_or_else(|| {
                        de::Error::custom(format!(
                            "`{type_path}` does not `#[reflect(ImageOperation)]`"
                        ))
                    })?;
                let op = reflect_op
                    .get_boxed(value)
                    .expect("should be able to downcast value into `ImageOperation`");

                Ok(op)
            }
        }

        struct HandleProcessor<'a, 'b> {
            load_context: &'a mut LoadContext<'b>,
        }

        impl ReflectDeserializerProcessor for HandleProcessor<'_, '_> {
            fn try_deserialize<'de, D>(
                &mut self,
                registration: &TypeRegistration,
                _registry: &TypeRegistry,
                deserializer: D,
            ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let Some(reflect_handle) = registration.data::<ReflectHandle>() else {
                    // This isn't a handle - use the default deserialization method.
                    return Ok(Err(deserializer));
                };

                let asset_type_id = reflect_handle.asset_type_id();
                let asset_path = deserializer.deserialize_str(AssetPathVisitor)?;
                let untyped_handle = self
                    .load_context
                    .loader()
                    .with_dynamic_type(asset_type_id)
                    .load(asset_path);
                let typed_handle = reflect_handle.typed(untyped_handle);
                Ok(Ok(typed_handle.into_partial_reflect()))
            }
        }

        struct AssetPathVisitor;

        impl<'de> Visitor<'de> for AssetPathVisitor {
            type Value = AssetPath<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an asset path")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                AssetPath::try_parse(v)
                    .map_err(|err| de::Error::custom(format!("invalid asset path: {err}")))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                AssetPath::try_parse(v)
                    .map(AssetPath::into_owned)
                    .map_err(|err| de::Error::custom(format!("invalid asset path: {err}")))
            }
        }

        deserializer.deserialize_seq(PipelineVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
        })
    }
}

// Asset loader implementation

#[derive(Debug)]
struct ImagePipelineLoader {
    type_registry: TypeRegistryArc,
}

#[derive(Debug, Error)]
enum ImagePipelineLoaderError {
    #[error("failed to read bytes")]
    ReadBytes(#[from] io::Error),
    #[error("failed to make RON deserializer")]
    MakeRonDeserializer(#[from] ron::error::SpannedError),
    #[error("failed to parse RON: {0:?}")]
    ParseRon(#[from] ron::Error),
}

impl FromWorld for ImagePipelineLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>();
        Self {
            type_registry: type_registry.0.clone(),
        }
    }
}

impl AssetLoader for ImagePipelineLoader {
    type Asset = ImagePipeline;
    type Settings = ();
    type Error = ImagePipelineLoaderError;

    fn extensions(&self) -> &[&str] {
        &["imgpipeline.ron"]
    }

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let mut ron_deserializer = ron::Deserializer::from_bytes(&bytes)?;
        // Put this into its own block so that the `read()` lock isn't held for
        // the entire scope.
        let pipeline = {
            ImagePipelineDeserializer {
                type_registry: &self.type_registry.read(),
                load_context,
            }
            .deserialize(&mut ron_deserializer)
        }?;

        Ok(pipeline)
    }
}

// App logic

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_asset::<ImagePipeline>()
        .init_asset_loader::<ImagePipelineLoader>()
        .init_resource::<DemoImagePipeline>()
        .register_type::<Load>()
        .register_type::<Invert>()
        .register_type::<Blur>()
        .add_systems(Startup, setup)
        .add_systems(Update, make_demo_image)
        .run()
}

#[derive(Debug, Default, Resource)]
struct DemoImagePipeline(Handle<ImagePipeline>);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut demo_image_pipeline: ResMut<DemoImagePipeline>,
    mut _todo: ResMut<Assets<ImagePipeline>>,
) {
    demo_image_pipeline.0 = asset_server.load("data/demo.imgpipeline.ron");

    // draw the demo image
    commands.spawn(Camera2d);
    commands.spawn(SpriteBundle {
        texture: Handle::default(),
        ..default()
    });
}

/// Updates the demo image entity to render with the output of the
/// [`DemoImagePipeline`].
fn make_demo_image(
    mut demo_images: Query<&mut Handle<Image>>,
    image_pipeline_assets: Res<Assets<ImagePipeline>>,
    mut image_assets: ResMut<Assets<Image>>,
    demo_image_pipeline: Res<DemoImagePipeline>,
) {
    let Some(demo_image_pipeline) = image_pipeline_assets.get(&demo_image_pipeline.0) else {
        return;
    };

    let dyn_image = DynamicImage::new(1, 1, ColorType::Rgba8);
    let dyn_image = demo_image_pipeline.apply(dyn_image, &image_assets);
    let image = Image::from_dynamic(dyn_image, true, RenderAssetUsages::RENDER_WORLD);
    let image_handle = image_assets.add(image);

    for mut demo_image in &mut demo_images {
        *demo_image = image_handle.clone();
    }
}
