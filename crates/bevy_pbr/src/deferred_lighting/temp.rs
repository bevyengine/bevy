// Recursive expansion of AsBindGroup macro
// =========================================

impl AsBindGroup for StandardMaterial {
    type Data = StandardMaterialKey;
    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &bevy_render::renderer::RenderDevice,
        images: &bevy_render::render_asset::RenderAssets<bevy_render::texture::Image>,
        fallback_image: &bevy_render::texture::FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        let bindings = vec![
            {
                use AsBindGroupShaderType;
                let mut buffer = encase::UniformBuffer::new(Vec::new());
                let converted: StandardMaterialUniform = self.as_bind_group_shader_type(images);
                buffer.write(&converted).unwrap();
                OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                    &BufferInitDescriptor {
                        label: None,
                        usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                        contents: buffer.as_ref(),
                    },
                ))
            },
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.base_color_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.base_color_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.emissive_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.emissive_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.metallic_roughness_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.metallic_roughness_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.normal_map_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.normal_map_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.occlusion_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.occlusion_texture).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
            OwnedBindingResource::TextureView({
                let handle: =
                    (&self.depth_map).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .texture_view
                        .clone()
                } else {
                    fallback_image.texture_view.clone()
                }
            }),
            OwnedBindingResource::Sampler({
                let handle: =
                    (&self.depth_map).into();
                if let Some(handle) = handle {
                    images
                        .get(handle)
                        .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?
                        .sampler
                        .clone()
                } else {
                    fallback_image.sampler.clone()
                }
            }),
        ];
        let bind_group = {
            let descriptor = BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0u32,
                        resource: bindings[0usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 1u32,
                        resource: bindings[1usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 2u32,
                        resource: bindings[2usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 3u32,
                        resource: bindings[3usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 4u32,
                        resource: bindings[4usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 5u32,
                        resource: bindings[5usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 6u32,
                        resource: bindings[6usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 9u32,
                        resource: bindings[7usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 10u32,
                        resource: bindings[8usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 7u32,
                        resource: bindings[9usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 8u32,
                        resource: bindings[10usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 11u32,
                        resource: bindings[11usize].get_binding(),
                    },
                    BindGroupEntry {
                        binding: 12u32,
                        resource: bindings[12usize].get_binding(),
                    },
                ],
                label: None,
                layout: &layout,
            };
            render_device.create_bind_group(&descriptor)
        };
        Ok(PreparedBindGroup {
            bindings,
            bind_group,
            data: self.into(),
        })
    }
    fn bind_group_layout(render_device: &bevy_render::renderer::RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0u32,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(<StandardMaterialUniform as ShaderType>::min_size()),
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
                BindGroupLayoutEntry {
                    binding: 3u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 9u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 10u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 11u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 12u32,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        })
    }
}
