use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroupError, PreparedBindGroup, *},
        renderer::RenderDevice,
        texture::{FallbackImage, ImageSampler},
    },
};
use std::num::NonZeroU32;

/// This example illustrates how to bind several textures in one
/// `binding_array<texture<f32>>` shader binding slot and sample non-uniformly.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<BindlessMaterial>::default())
        .init_resource::<ColorTextures>()
        .add_startup_system(setup)
        .run();
}

const MAX_TEXTURE_COUNT: usize = 16;

#[derive(Resource, Deref)]
struct ColorTextures(Vec<Handle<Image>>);

impl FromWorld for ColorTextures {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();

        // Create 16 textures with different color gradients
        let handles = (1..=MAX_TEXTURE_COUNT)
            .map(|id| {
                let mut pixel = vec![(256 / id - 1) as u8; 64];
                for y in 0..3 {
                    for x in 0..3 {
                        pixel[16 * y + 4 * x + 1] = (256 / (y + 1) - 1) as u8;
                        pixel[16 * y + 4 * x + 2] = (256 / (x + 1) - 1) as u8;
                        pixel[16 * y + 4 * x + 3] = 255;
                    }
                }

                let mut image = Image::new_fill(
                    Extent3d {
                        width: 4,
                        height: 4,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &pixel[..],
                    TextureFormat::Rgba8Unorm,
                );
                image.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                    address_mode_u: AddressMode::Repeat,
                    address_mode_v: AddressMode::Repeat,
                    address_mode_w: AddressMode::Repeat,
                    ..Default::default()
                });
                images.add(image)
            })
            .collect();

        Self(handles)
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    color_textures: Res<ColorTextures>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(BindlessMaterial {
            textures: color_textures.clone(),
        }),
        ..Default::default()
    });
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "8dd2b424-45a2-4a53-ac29-7ce356b2d5fe"]
struct BindlessMaterial {
    textures: Vec<Handle<Image>>,
}

impl AsBindGroup for BindlessMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        let textures = vec![&fallback_image.texture_view; MAX_TEXTURE_COUNT];
        let samplers = vec![&fallback_image.sampler; MAX_TEXTURE_COUNT];

        // Convert bevy's resource types to wgpu's references
        let mut textures: Vec<_> = textures.into_iter().map(|texture| &**texture).collect();
        let mut samplers: Vec<_> = samplers.into_iter().map(|sampler| &**sampler).collect();

        // Fill in up to the first `MAX_TEXTURE_COUNT` textures and samplers to the arrays
        for (id, image) in self
            .textures
            .iter()
            .filter_map(|handle| images.get(handle))
            .take(MAX_TEXTURE_COUNT)
            .enumerate()
        {
            textures[id] = &*image.texture_view;
            samplers[id] = &*image.sampler;
        }

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: "bindless_material_bind_group".into(),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&textures[..]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::SamplerArray(&samplers[..]),
                },
            ],
        });

        Ok(PreparedBindGroup {
            bindings: vec![],
            bind_group,
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout
    where
        Self: Sized,
    {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "bindless_material_layout".into(),
            entries: &[
                // @group(1) @binding(0) var textures: binding_array<texture_2d<f32>>;
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
                // @group(1) @binding(1) var samplers: binding_array<sampler>;
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: NonZeroU32::new(MAX_TEXTURE_COUNT as u32),
                },
            ],
        })
    }
}

impl Material for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/texture_binding_array.wgsl".into()
    }
}
