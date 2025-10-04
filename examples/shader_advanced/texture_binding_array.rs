//! A shader that binds several textures onto one
//! `binding_array<texture<f32>>` shader binding slot and sample non-uniformly.

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypePath,
    render::{
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{sampler, texture_2d},
            *,
        },
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
        RenderApp, RenderStartup,
    },
    shader::ShaderRef,
};
use std::{num::NonZero, process::exit};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/texture_binding_array.wgsl";

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
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, verify_required_features);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // load 16 textures
    let textures: Vec<_> = TILE_ID
        .iter()
        .map(|id| asset_server.load(format!("textures/rpg/tiles/generic-rpg-tile{id:0>2}.png")))
        .collect();

    // a cube with multiple textures
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(BindlessMaterial { textures })),
    ));
}

fn verify_required_features(render_device: Res<RenderDevice>) {
    // Check if the device support the required feature. If not, exit the example. In a real
    // application, you should setup a fallback for the missing feature
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

#[derive(Asset, TypePath, Debug, Clone)]
struct BindlessMaterial {
    textures: Vec<Handle<Image>>,
}

impl AsBindGroup for BindlessMaterial {
    type Data = ();

    type Param = (SRes<RenderAssets<GpuImage>>, SRes<FallbackImage>);

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
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
            bindings: BindingResources(vec![]),
            bind_group,
        })
    }

    fn bind_group_data(&self) -> Self::Data {}

    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _param: &mut SystemParamItem<'_, '_, Self::Param>,
        _force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        // We implement `as_bind_group`` directly because bindless texture
        // arrays can't be owned.
        // Or rather, they can be owned, but then you can't make a `&'a [&'a
        // TextureView]` from a vec of them in `get_binding()`.
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(_: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        BindGroupLayoutEntries::with_indices(
            // The layout entries will only be visible in the fragment stage
            ShaderStages::FRAGMENT,
            (
                // Screen texture
                //
                // @group(#{MATERIAL_BIND_GROUP}) @binding(0) var textures: binding_array<texture_2d<f32>>;
                (
                    0,
                    texture_2d(TextureSampleType::Float { filterable: true })
                        .count(NonZero::<u32>::new(MAX_TEXTURE_COUNT as u32).unwrap()),
                ),
                // Sampler
                //
                // @group(#{MATERIAL_BIND_GROUP}) @binding(1) var nearest_sampler: sampler;
                //
                // Note: as with textures, multiple samplers can also be bound
                // onto one binding slot:
                //
                // ```
                // sampler(SamplerBindingType::Filtering)
                //     .count(NonZero::<u32>::new(MAX_TEXTURE_COUNT as u32).unwrap()),
                // ```
                //
                // One may need to pay attention to the limit of sampler binding
                // amount on some platforms.
                (1, sampler(SamplerBindingType::Filtering)),
            ),
        )
        .to_vec()
    }
}

impl Material for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
