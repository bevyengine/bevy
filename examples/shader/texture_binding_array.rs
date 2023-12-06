//! A shader that binds several textures onto one
//! `binding_array<texture<f32>>` shader binding slot and sample non-uniformly.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice,
        texture::FallbackImage, RenderApp,
    },
};
use std::{num::NonZeroU32, process::exit};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        GpuFeatureSupportChecker,
        MaterialPlugin::<BindlessMaterial>::default(),
    ))
    .add_systems(Startup, setup)
    .run();
}

const MAX_TEXTURE_COUNT: usize = 16;
const TILE_ID: [usize; 16] = [
    19, 23, 4, 33, 12, 69, 30, 48, 10, 65, 40, 47, 57, 41, 44, 46,
];

struct GpuFeatureSupportChecker;

impl Plugin for GpuFeatureSupportChecker {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world.resource::<RenderDevice>();

        // Check if the device support the required feature. If not, exit the example.
        // In a real application, you should setup a fallback for the missing feature
        if !render_device
            .features()
            .contains(WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
        {
            error!(
                "Render device doesn't support feature \
SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING, \
which is required for texture binding arrays"
            );
            exit(1);
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    // load 16 textures
    let textures: Vec<_> = TILE_ID
        .iter()
        .map(|id| asset_server.load(format!("textures/rpg/tiles/generic-rpg-tile{id:0>2}.png")))
        .collect();

    // a cube with multiple textures
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(BindlessMaterial { textures }),
        ..Default::default()
    });
}

#[derive(Asset, TypePath, Debug, Clone)]
struct BindlessMaterial {
    textures: Vec<Handle<Image>>,
}

impl AsBindGroup for BindlessMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        image_assets: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        // retrieve the render resources from handles
        let mut images = vec![];
        for handle in self.textures.iter().take(MAX_TEXTURE_COUNT) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        let fallback_image = &fallback_image.d2;

        let textures = vec![&fallback_image.texture_view; MAX_TEXTURE_COUNT];

        // convert bevy's resource types to WGPU's references
        let mut textures: Vec<_> = textures.into_iter().map(|texture| &**texture).collect();

        // fill in up to the first `MAX_TEXTURE_COUNT` textures and samplers to the arrays
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        let bind_group = render_device.create_bind_group(
            "bindless_material_bind_group",
            layout,
            &BindGroupEntries::sequential((&textures[..], &fallback_image.sampler)),
        );

        Ok(PreparedBindGroup {
            bindings: vec![],
            bind_group,
            data: (),
        })
    }

    fn unprepared_bind_group(
        &self,
        _: &BindGroupLayout,
        _: &RenderDevice,
        _: &RenderAssets<Image>,
        _: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // we implement as_bind_group directly because
        panic!("bindless texture arrays can't be owned")
        // or rather, they can be owned, but then you can't make a `&'a [&'a TextureView]` from a vec of them in get_binding().
    }

    fn bind_group_layout_entries(_: &RenderDevice) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        vec![
            // @group(2) @binding(0) var textures: binding_array<texture_2d<f32>>;
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: NonZeroU32::new(MAX_TEXTURE_COUNT as u32),
            },
            // @group(2) @binding(1) var nearest_sampler: sampler;
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
                // Note: as textures, multiple samplers can also be bound onto one binding slot.
                // One may need to pay attention to the limit of sampler binding amount on some platforms.
                // count: NonZeroU32::new(MAX_TEXTURE_COUNT as u32),
            },
        ]
    }
}

impl Material for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/texture_binding_array.wgsl".into()
    }
}
