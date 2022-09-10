//! A shader and a material that uses it using Bevy's Material API via manual implementation
//! See `shader_material` example for a higher level implementation.

use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::RenderAssets,
        render_resource::{
            encase::UniformBuffer, AsBindGroup, AsBindGroupError, BindGroupDescriptor,
            BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferInitDescriptor, BufferUsages,
            OwnedBindingResource, PreparedBindGroup, SamplerBindingType, ShaderRef, ShaderStages,
            ShaderType, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // cube
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {
            color: Color::BLUE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        }),
        ..default()
    });

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}

/// This is the struct that will be passed to your shader
/// Specific order of declaration is not relevant, but "binding" identifier values are.
/// Defined in `AsBindGroup` implementation, they should match their corresponding shader @binding value.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
struct CustomMaterial {
    color: Color,
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

impl AsBindGroup for CustomMaterial {
    type Data = ();
    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        // Step 1: retrieve information from our extracted type.
        let color_texture = {
            let color_texture_handle: Option<&Handle<Image>> = (&self.color_texture).into();
            if let Some(handle) = color_texture_handle {
                images
                    .get(handle)
                    .ok_or(AsBindGroupError::RetryNextUpdate)?
            } else {
                fallback_image
            }
        };

        // Step 2: set specific bind ground type depending on exported information
        let color = {
            let mut buffer = UniformBuffer::new(Vec::new());
            buffer.write(&self.color).unwrap();
            OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: None,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    contents: buffer.as_ref(),
                },
            ))
        };
        let texture_view = OwnedBindingResource::TextureView(color_texture.texture_view.clone());
        let color_texture_sampler = OwnedBindingResource::Sampler(color_texture.sampler.clone());

        // Step 3: set binding group ids.
        let bind_group = {
            let descriptor = BindGroupDescriptor {
                // Specific order within entries is not relevant,
                // but the `binding` value should match its corresponding @binding from the shader.
                entries: &[
                    BindGroupEntry {
                        binding: 0u32,
                        resource: { color.get_binding() },
                    },
                    BindGroupEntry {
                        binding: 1u32,
                        resource: texture_view.get_binding(),
                    },
                    BindGroupEntry {
                        binding: 2u32,
                        resource: color_texture_sampler.get_binding(),
                    },
                ],
                label: None,
                layout,
            };
            render_device.create_bind_group(&descriptor)
        };

        // Step 4: construct PreparedBindGroup
        Ok(PreparedBindGroup {
            bindings: vec![color, texture_view, color_texture_sampler],
            bind_group,
            data: (),
        })
    }

    fn bind_group_layout(
        render_device: &bevy_internal::render::renderer::RenderDevice,
    ) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            // Specific order within entries is not relevant,
            // but the `binding` value should match its corresponding @binding from the shader.
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0u32,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Color as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        })
    }
}
