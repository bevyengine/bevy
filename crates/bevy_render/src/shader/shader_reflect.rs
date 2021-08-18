use crate::{
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, BindingShaderStage, InputStepMode,
        UniformProperty, VertexAttribute, VertexBufferLayout, VertexFormat,
    },
    shader::{ShaderLayout, GL_FRONT_FACING, GL_INSTANCE_INDEX, GL_VERTEX_INDEX},
    texture::{TextureSampleType, TextureViewDimension},
};
use bevy_core::cast_slice;
use spirv_reflect::{
    types::{
        ReflectDescriptorBinding, ReflectDescriptorSet, ReflectDescriptorType, ReflectDimension,
        ReflectShaderStageFlags, ReflectTypeDescription, ReflectTypeFlags,
    },
    ShaderModule,
};

impl ShaderLayout {
    pub fn from_spirv(spirv_data: &[u32], bevy_conventions: bool) -> ShaderLayout {
        match ShaderModule::load_u8_data(cast_slice(spirv_data)) {
            Ok(ref mut module) => {
                // init
                let entry_point_name = module.get_entry_point_name();
                let shader_stage = module.get_shader_stage();
                let mut bind_groups = Vec::new();
                for descriptor_set in module.enumerate_descriptor_sets(None).unwrap() {
                    let bind_group = reflect_bind_group(&descriptor_set, shader_stage);
                    bind_groups.push(bind_group);
                }

                // obtain attribute descriptors from reflection
                let mut vertex_attributes = Vec::new();
                for input_variable in module.enumerate_input_variables(None).unwrap() {
                    if input_variable.name == GL_VERTEX_INDEX
                        || input_variable.name == GL_INSTANCE_INDEX
                        || input_variable.name == GL_FRONT_FACING
                    {
                        continue;
                    }
                    // reflect vertex attribute descriptor and record it
                    vertex_attributes.push(VertexAttribute {
                        name: input_variable.name.clone().into(),
                        format: reflect_vertex_format(
                            input_variable.type_description.as_ref().unwrap(),
                        ),
                        offset: 0,
                        shader_location: input_variable.location,
                    });
                }

                vertex_attributes.sort_by(|a, b| a.shader_location.cmp(&b.shader_location));

                let mut vertex_buffer_layout = Vec::new();
                for vertex_attribute in vertex_attributes.drain(..) {
                    let mut instance = false;
                    // obtain buffer name and instancing flag
                    let current_buffer_name = {
                        if bevy_conventions {
                            if vertex_attribute.name == GL_VERTEX_INDEX {
                                GL_VERTEX_INDEX.to_string()
                            } else {
                                instance = vertex_attribute.name.starts_with("I_");
                                vertex_attribute.name.to_string()
                            }
                        } else {
                            "DefaultVertex".to_string()
                        }
                    };

                    // create a new buffer descriptor, per attribute!
                    vertex_buffer_layout.push(VertexBufferLayout {
                        attributes: vec![vertex_attribute],
                        name: current_buffer_name.into(),
                        step_mode: if instance {
                            InputStepMode::Instance
                        } else {
                            InputStepMode::Vertex
                        },
                        stride: 0,
                    });
                }

                ShaderLayout {
                    bind_groups,
                    vertex_buffer_layout,
                    entry_point: entry_point_name,
                }
            }
            Err(err) => panic!("Failed to reflect shader layout: {:?}.", err),
        }
    }
}

fn reflect_bind_group(
    descriptor_set: &ReflectDescriptorSet,
    shader_stage: ReflectShaderStageFlags,
) -> BindGroupDescriptor {
    let mut bindings = Vec::new();
    for descriptor_binding in descriptor_set.bindings.iter() {
        let binding = reflect_binding(descriptor_binding, shader_stage);
        bindings.push(binding);
    }

    BindGroupDescriptor::new(descriptor_set.set, bindings)
}

fn reflect_dimension(type_description: &ReflectTypeDescription) -> TextureViewDimension {
    match type_description.traits.image.dim {
        ReflectDimension::Type1d => TextureViewDimension::D1,
        ReflectDimension::Type2d => TextureViewDimension::D2,
        ReflectDimension::Type3d => TextureViewDimension::D3,
        ReflectDimension::Cube => TextureViewDimension::Cube,
        dimension => panic!("Unsupported image dimension: {:?}.", dimension),
    }
}

fn reflect_binding(
    binding: &ReflectDescriptorBinding,
    shader_stage: ReflectShaderStageFlags,
) -> BindingDescriptor {
    let type_description = binding.type_description.as_ref().unwrap();
    let (name, bind_type) = match binding.descriptor_type {
        ReflectDescriptorType::UniformBuffer => (
            &type_description.type_name,
            BindType::Uniform {
                has_dynamic_offset: false,
                property: reflect_uniform(type_description),
            },
        ),
        ReflectDescriptorType::SampledImage => (
            &binding.name,
            BindType::Texture {
                view_dimension: reflect_dimension(type_description),
                sample_type: TextureSampleType::Float { filterable: true },
                multisampled: false,
            },
        ),
        ReflectDescriptorType::StorageBuffer => (
            &type_description.type_name,
            BindType::StorageBuffer {
                has_dynamic_offset: false,
                readonly: true,
            },
        ),
        // TODO: detect comparison "true" case: https://github.com/gpuweb/gpuweb/issues/552
        // TODO: detect filtering "true" case
        ReflectDescriptorType::Sampler => (
            &binding.name,
            BindType::Sampler {
                comparison: false,
                filtering: true,
            },
        ),
        _ => {
            let ReflectDescriptorBinding {
                descriptor_type,
                name,
                set,
                binding,
                ..
            } = binding;
            panic!(
                "Unsupported shader bind type {:?} (name '{}', set {}, binding {})",
                descriptor_type, name, set, binding
            );
        }
    };

    let shader_stage = match shader_stage {
        ReflectShaderStageFlags::COMPUTE => BindingShaderStage::COMPUTE,
        ReflectShaderStageFlags::VERTEX => BindingShaderStage::VERTEX,
        ReflectShaderStageFlags::FRAGMENT => BindingShaderStage::FRAGMENT,
        _ => panic!("Only one specified shader stage is supported."),
    };

    BindingDescriptor {
        index: binding.binding,
        bind_type,
        name: name.to_string(),
        shader_stage,
    }
}

#[derive(Debug)]
enum NumberType {
    Int,
    UInt,
    Float,
}

fn reflect_uniform(type_description: &ReflectTypeDescription) -> UniformProperty {
    if type_description
        .type_flags
        .contains(ReflectTypeFlags::STRUCT)
    {
        reflect_uniform_struct(type_description)
    } else {
        reflect_uniform_numeric(type_description)
    }
}

fn reflect_uniform_struct(type_description: &ReflectTypeDescription) -> UniformProperty {
    let mut properties = Vec::new();
    for member in type_description.members.iter() {
        properties.push(reflect_uniform(member));
    }

    UniformProperty::Struct(properties)
}

fn reflect_uniform_numeric(type_description: &ReflectTypeDescription) -> UniformProperty {
    let traits = &type_description.traits;
    let number_type = if type_description.type_flags.contains(ReflectTypeFlags::INT) {
        match traits.numeric.scalar.signedness {
            0 => NumberType::UInt,
            1 => NumberType::Int,
            signedness => panic!("Unexpected signedness {}.", signedness),
        }
    } else if type_description
        .type_flags
        .contains(ReflectTypeFlags::FLOAT)
    {
        NumberType::Float
    } else {
        panic!("Unexpected type flag {:?}.", type_description.type_flags);
    };

    // TODO: handle scalar width here

    if type_description
        .type_flags
        .contains(ReflectTypeFlags::MATRIX)
    {
        match (
            number_type,
            traits.numeric.matrix.column_count,
            traits.numeric.matrix.row_count,
        ) {
            (NumberType::Float, 3, 3) => UniformProperty::Mat3,
            (NumberType::Float, 4, 4) => UniformProperty::Mat4,
            (number_type, column_count, row_count) => panic!(
                "unexpected uniform property matrix format {:?} {}x{}",
                number_type, column_count, row_count
            ),
        }
    } else {
        match (number_type, traits.numeric.vector.component_count) {
            (NumberType::UInt, 0) => UniformProperty::UInt,
            (NumberType::Int, 0) => UniformProperty::Int,
            (NumberType::Int, 2) => UniformProperty::IVec2,
            (NumberType::Float, 0) => UniformProperty::Float,
            (NumberType::Float, 2) => UniformProperty::Vec2,
            (NumberType::Float, 3) => UniformProperty::Vec3,
            (NumberType::Float, 4) => UniformProperty::Vec4,
            (NumberType::UInt, 4) => UniformProperty::UVec4,
            (number_type, component_count) => panic!(
                "unexpected uniform property format {:?} {}",
                number_type, component_count
            ),
        }
    }
}

fn reflect_vertex_format(type_description: &ReflectTypeDescription) -> VertexFormat {
    let traits = &type_description.traits;
    let number_type = if type_description.type_flags.contains(ReflectTypeFlags::INT) {
        match traits.numeric.scalar.signedness {
            0 => NumberType::UInt,
            1 => NumberType::Int,
            signedness => panic!("Unexpected signedness {}.", signedness),
        }
    } else if type_description
        .type_flags
        .contains(ReflectTypeFlags::FLOAT)
    {
        NumberType::Float
    } else {
        panic!("Unexpected type flag {:?}.", type_description.type_flags);
    };

    let width = traits.numeric.scalar.width;

    match (number_type, traits.numeric.vector.component_count, width) {
        (NumberType::UInt, 2, 8) => VertexFormat::Uint8x2,
        (NumberType::UInt, 4, 8) => VertexFormat::Uint8x4,
        (NumberType::Int, 2, 8) => VertexFormat::Sint8x2,
        (NumberType::Int, 4, 8) => VertexFormat::Sint8x4,
        (NumberType::UInt, 2, 16) => VertexFormat::Uint16x2,
        (NumberType::UInt, 4, 16) => VertexFormat::Uint16x4,
        (NumberType::Int, 2, 16) => VertexFormat::Sint16x2,
        (NumberType::Int, 8, 16) => VertexFormat::Sint16x4,
        (NumberType::Float, 2, 16) => VertexFormat::Float16x2,
        (NumberType::Float, 4, 16) => VertexFormat::Float16x4,
        (NumberType::Float, 0, 32) => VertexFormat::Float32,
        (NumberType::Float, 2, 32) => VertexFormat::Float32x2,
        (NumberType::Float, 3, 32) => VertexFormat::Float32x3,
        (NumberType::Float, 4, 32) => VertexFormat::Float32x4,
        (NumberType::UInt, 0, 32) => VertexFormat::Uint32,
        (NumberType::UInt, 2, 32) => VertexFormat::Uint32x2,
        (NumberType::UInt, 3, 32) => VertexFormat::Uint32x3,
        (NumberType::UInt, 4, 32) => VertexFormat::Uint32x4,
        (NumberType::Int, 0, 32) => VertexFormat::Sint32,
        (NumberType::Int, 2, 32) => VertexFormat::Sint32x2,
        (NumberType::Int, 3, 32) => VertexFormat::Sint32x3,
        (NumberType::Int, 4, 32) => VertexFormat::Sint32x4,
        (number_type, component_count, width) => panic!(
            "unexpected uniform property format {:?} {} {}",
            number_type, component_count, width
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::{Shader, ShaderStage};

    impl VertexBufferLayout {
        pub fn test_zero_stride(mut self) -> VertexBufferLayout {
            self.stride = 0;
            self
        }
    }
    #[test]
    fn test_reflection() {
        let vertex_shader = Shader::from_glsl(
            ShaderStage::Vertex,
            r#"
            #version 450
            layout(location = 0) in vec4 Vertex_Position;
            layout(location = 1) in uvec4 Vertex_Normal;
            layout(location = 2) in uvec4 I_TestInstancing_Property;

            layout(location = 0) out vec4 v_Position;
            layout(set = 0, binding = 0) uniform CameraViewProj {
                mat4 ViewProj;
            };
            layout(set = 1, binding = 0) uniform texture2D Texture;

            void main() {
                v_Position = Vertex_Position;
                gl_Position = ViewProj * v_Position;
            }
        "#,
        )
        .get_spirv_shader(None)
        .unwrap();

        let layout = vertex_shader.reflect_layout(true).unwrap();
        assert_eq!(
            layout,
            ShaderLayout {
                entry_point: "main".into(),
                vertex_buffer_layout: vec![
                    VertexBufferLayout::new_from_attribute(
                        VertexAttribute {
                            name: "Vertex_Position".into(),
                            format: VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        InputStepMode::Vertex
                    )
                    .test_zero_stride(),
                    VertexBufferLayout::new_from_attribute(
                        VertexAttribute {
                            name: "Vertex_Normal".into(),
                            format: VertexFormat::Uint32x4,
                            offset: 0,
                            shader_location: 1,
                        },
                        InputStepMode::Vertex
                    )
                    .test_zero_stride(),
                    VertexBufferLayout::new_from_attribute(
                        VertexAttribute {
                            name: "I_TestInstancing_Property".into(),
                            format: VertexFormat::Uint32x4,
                            offset: 0,
                            shader_location: 2,
                        },
                        InputStepMode::Instance
                    )
                    .test_zero_stride(),
                ],
                bind_groups: vec![
                    BindGroupDescriptor::new(
                        0,
                        vec![BindingDescriptor {
                            index: 0,
                            name: "CameraViewProj".into(),
                            bind_type: BindType::Uniform {
                                has_dynamic_offset: false,
                                property: UniformProperty::Struct(vec![UniformProperty::Mat4]),
                            },
                            shader_stage: BindingShaderStage::VERTEX,
                        }]
                    ),
                    BindGroupDescriptor::new(
                        1,
                        vec![BindingDescriptor {
                            index: 0,
                            name: "Texture".into(),
                            bind_type: BindType::Texture {
                                multisampled: false,
                                view_dimension: TextureViewDimension::D2,
                                sample_type: TextureSampleType::Float { filterable: true }
                            },
                            shader_stage: BindingShaderStage::VERTEX,
                        }]
                    ),
                ]
            }
        );
    }
}
