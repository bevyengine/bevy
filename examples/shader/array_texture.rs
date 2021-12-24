use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::MaterialPipeline,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
    },
};

/// This example illustrates how to create a texture for use with a texture2DArray shader uniform
/// variable.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<ArrayTextureMaterial>::default())
        .add_startup_system(setup)
        .add_system(create_array_texture)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Start loading the texture.
    commands.insert_resource(LoadingTexture(Some(
        asset_server.load("textures/array_texture.png"),
    )));

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

struct LoadingTexture(Option<Handle<Image>>);

fn create_array_texture(
    mut commands: Commands,
    mut loading_texture: ResMut<LoadingTexture>,
    mut textures: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
) {
    let (handle, texture) = match loading_texture.0.as_ref() {
        Some(handle) => {
            if let Some(texture) = textures.get_mut(handle) {
                (loading_texture.0.take().unwrap(), texture)
            } else {
                return;
            }
        }
        None => return,
    };

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    texture.reinterpret_stacked_2d_as_array(array_layers);
    texture.sampler_descriptor.address_mode_u = AddressMode::Repeat;
    texture.sampler_descriptor.address_mode_v = AddressMode::Repeat;

    // Spawn a cube that's shaded using the array texture.
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(ArrayTextureMaterial {
            array_texture: handle,
        }),
        ..Default::default()
    });
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "9c5a0ddf-1eaf-41b4-9832-ed736fd26af3"]
struct ArrayTextureMaterial {
    array_texture: Handle<Image>,
}

#[derive(Clone)]
pub struct GpuArrayTextureMaterial {
    bind_group: BindGroup,
}

impl RenderAsset for ArrayTextureMaterial {
    type ExtractedAsset = ArrayTextureMaterial;
    type PreparedAsset = GpuArrayTextureMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<Self>>,
        SRes<RenderAssets<Image>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (array_texture_texture_view, array_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, &Some(extracted_asset.array_texture.clone()))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(array_texture_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(array_texture_sampler),
                },
            ],
            label: Some("array_texture_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuArrayTextureMaterial { bind_group })
    }
}

impl Material for ArrayTextureMaterial {
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/array_texture.wgsl"))
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/array_texture.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Array Texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Array Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        })
    }
}
