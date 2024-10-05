//! Let's imagine we want to make a system where we can take an image, pass it
//! through a series of operations (a pipeline), and get back a new transformed
//! image.
//!
//! We want to define the pipeline using an asset, and the operations should be
//! fully dynamic - users should be able to register their own operation types,
//! and the asset loader can deserialize them without any extra setup.

use std::fmt::Debug;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext, RenderAssetUsages},
    prelude::*,
    reflect::TypeRegistryArc,
};
use image::{ColorType, DynamicImage};
use thiserror::Error;

/// Series of [`ImageOperation`]s which may be applied to an image.
#[derive(Debug, Asset)]
// we can't automatically derive Reflect yet - see <https://github.com/bevyengine/bevy/pull/15532>
// so we manually implement the Reflect traits below.
// #[derive(Reflect)]
#[derive(TypePath)]
struct ImagePipeline {
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

/// Applies an operation to an image in the pipeline.
#[reflect_trait]
trait ImageOperation: Debug + Send + Sync + Reflect {
    /// Applies the operation.
    fn apply(&self, ctx: &mut ImageOperationContext<'_>);
}

struct ImageOperationContext<'a> {
    current: DynamicImage,
    image_assets: &'a Assets<Image>,
}

// operation implementations

/// Overwrites the current image with another image asset.
#[derive(Debug, Clone, Reflect)]
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
struct Invert;

impl ImageOperation for Invert {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>) {
        ctx.current.invert();
    }
}

/// Applies a Gaussian blur to the image.
#[derive(Debug, Clone, Reflect)]
struct Blur {
    /// Blur intensity.
    sigma: f32,
}

impl ImageOperation for Blur {
    fn apply(&self, ctx: &mut ImageOperationContext<'_>) {
        ctx.current = ctx.current.blur(self.sigma);
    }
}

// asset loader

#[derive(Debug)]
struct ImagePipelineLoader {
    type_registry: TypeRegistryArc,
}

#[derive(Debug, Error)]
enum ImagePipelineLoaderError {}

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

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // let mut bytes = Vec::new();
        // reader.read_to_end(&mut bytes).await?;

        // let mut ron_deserializer = ron::Deserializer::from_bytes(&bytes)?;

        todo!()
    }
}

// app logic

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_asset::<ImagePipeline>()
        .init_asset_loader::<ImagePipelineLoader>()
        .init_resource::<DemoImagePipeline>()
        .add_systems(Startup, setup)
        .add_systems(Update, make_demo_image)
        .run()
}

#[derive(Debug, Default, Resource)]
struct DemoImagePipeline(Handle<ImagePipeline>);

#[derive(Debug, Component)]
struct DemoImage;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut demo_image_pipeline: ResMut<DemoImagePipeline>,
    mut _todo: ResMut<Assets<ImagePipeline>>,
) {
    // demo_image_pipeline.0 = asset_server.load("data/demo_image_pipeline.imgpipe.ron");
    demo_image_pipeline.0 = _todo.add(ImagePipeline {
        ops: vec![
            Box::new(Load {
                image: asset_server.load("textures/Ryfjallet_cubemap.png"),
            }),
            Box::new(Invert),
            Box::new(Blur { sigma: 2.0 }),
        ],
    });

    // draw the demo image
    commands.spawn(Camera2d);
    commands.spawn((
        SpriteBundle {
            texture: Handle::default(),
            ..default()
        },
        DemoImage,
    ));
}

fn make_demo_image(
    mut demo_images: Query<&mut Handle<Image>>,
    image_pipeline_assets: Res<Assets<ImagePipeline>>,
    mut image_assets: ResMut<Assets<Image>>,
    demo_image_pipeline: Res<DemoImagePipeline>,
) {
    let Some(demo_image_pipeline) = image_pipeline_assets.get(&demo_image_pipeline.0) else {
        info!("Image pipeline not loaded yet");
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
