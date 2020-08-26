use crate::{
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, BindingShaderStage, InputStepMode,
        UniformProperty, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat,
    },
    texture::{TextureComponentType, TextureViewDimension},
};
use bevy_core::AsBytes;
use spirv_reflect::{
    types::{
        ReflectDescriptorBinding, ReflectDescriptorSet, ReflectDescriptorType, ReflectDimension,
        ReflectInterfaceVariable, ReflectShaderStageFlags, ReflectTypeDescription,
        ReflectTypeFlags,
    },
    ShaderModule,
};
use std::collections::HashSet;

/// Defines the memory layout of a shader
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderLayout {
    pub bind_groups: Vec<BindGroupDescriptor>,
    pub vertex_buffer_descriptors: Vec<VertexBufferDescriptor>,
    pub entry_point: String,
}

pub const GL_VERTEX_INDEX: &str = "gl_VertexIndex";

impl ShaderLayout {
    pub fn from_spirv(spirv_data: &[u32], bevy_conventions: bool) -> ShaderLayout {
        match ShaderModule::load_u8_data(spirv_data.as_bytes()) {
            Ok(ref mut module) => {
                let entry_point_name = module.get_entry_point_name();
                let shader_stage = module.get_shader_stage();
                let mut bind_groups = Vec::new();
                for descriptor_set in module.enumerate_descriptor_sets(None).unwrap() {
                    let bind_group = reflect_bind_group(&descriptor_set, shader_stage);
                    bind_groups.push(bind_group);
                }

                let mut vertex_attribute_descriptors = Vec::new();
                for input_variable in module.enumerate_input_variables(None).unwrap() {
                    let vertex_attribute_descriptor =
                        reflect_vertex_attribute_descriptor(&input_variable);
                    if vertex_attribute_descriptor.name == GL_VERTEX_INDEX {
                        continue;
                    }
                    vertex_attribute_descriptors.push(vertex_attribute_descriptor);
                }

                vertex_attribute_descriptors
                    .sort_by(|a, b| a.shader_location.cmp(&b.shader_location));

                let mut visited_buffer_descriptors = HashSet::new();
                let mut vertex_buffer_descriptors = Vec::new();
                let mut current_descriptor: Option<VertexBufferDescriptor> = None;
                for vertex_attribute_descriptor in vertex_attribute_descriptors.drain(..) {
                    let mut instance = false;
                    let current_buffer_name = {
                        if bevy_conventions {
                            if vertex_attribute_descriptor.name == GL_VERTEX_INDEX {
                                GL_VERTEX_INDEX.to_string()
                            } else {
                                let parts = vertex_attribute_descriptor
                                    .name
                                    .splitn(3, '_')
                                    .collect::<Vec<&str>>();
                                if parts.len() == 3 {
                                    if parts[0] == "I" {
                                        instance = true;
                                        parts[1].to_string()
                                    } else {
                                        parts[0].to_string()
                                    }
                                } else if parts.len() == 2 {
                                    parts[0].to_string()
                                } else {
                                    panic!("Vertex attributes must follow the form BUFFERNAME_PROPERTYNAME. For example: Vertex_Position");
                                }
                            }
                        } else {
                            "DefaultVertex".to_string()
                        }
                    };

                    if let Some(current) = current_descriptor.as_mut() {
                        if current.name == current_buffer_name {
                            current.attributes.push(vertex_attribute_descriptor);
                            continue;
                        } else if visited_buffer_descriptors.contains(&current_buffer_name) {
                            panic!("Vertex attribute buffer names must be consecutive.")
                        }
                    }

                    if let Some(current) = current_descriptor.take() {
                        visited_buffer_descriptors.insert(current.name.to_string());
                        vertex_buffer_descriptors.push(current);
                    }

                    current_descriptor = Some(VertexBufferDescriptor {
                        attributes: vec![vertex_attribute_descriptor],
                        name: current_buffer_name.into(),
                        step_mode: if instance {
                            InputStepMode::Instance
                        } else {
                            InputStepMode::Vertex
                        },
                        stride: 0,
                    })
                }

                if let Some(current) = current_descriptor.take() {
                    visited_buffer_descriptors.insert(current.name.to_string());
                    vertex_buffer_descriptors.push(current);
                }

                for vertex_buffer_descriptor in vertex_buffer_descriptors.iter_mut() {
                    calculate_offsets(vertex_buffer_descriptor);
                }

                ShaderLayout {
                    bind_groups,
                    vertex_buffer_descriptors,
                    entry_point: entry_point_name,
                }
            }
            Err(err) => panic!("Failed to reflect shader layout: {:?}", err),
        }
    }
}

fn calculate_offsets(vertex_buffer_descriptor: &mut VertexBufferDescriptor) {
    let mut offset = 0;
    for attribute in vertex_buffer_descriptor.attributes.iter_mut() {
        attribute.offset = offset;
        offset += attribute.format.get_size();
    }

    vertex_buffer_descriptor.stride = offset;
}

fn reflect_vertex_attribute_descriptor(
    input_variable: &ReflectInterfaceVariable,
) -> VertexAttributeDescriptor {
    VertexAttributeDescriptor {
        name: input_variable.name.clone().into(),
        format: reflect_vertex_format(input_variable.type_description.as_ref().unwrap()),
        offset: 0,
        shader_location: input_variable.location,
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
        dimension => panic!("unsupported image dimension: {:?}", dimension),
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
                dynamic: false,
                property: reflect_uniform(type_description),
            },
        ),
        ReflectDescriptorType::SampledImage => (
            &binding.name,
            BindType::SampledTexture {
                dimension: reflect_dimension(type_description),
                component_type: TextureComponentType::Float,
                multisampled: false,
            },
        ),
        ReflectDescriptorType::StorageBuffer => (
            &type_description.type_name,
            BindType::StorageBuffer {
                dynamic: false,
                readonly: true,
            },
        ),
        // TODO: detect comparison "true" case: https://github.com/gpuweb/gpuweb/issues/552
        ReflectDescriptorType::Sampler => (&binding.name, BindType::Sampler { comparison: false }),
        _ => panic!("unsupported bind type {:?}", binding.descriptor_type),
    };

    let mut shader_stage = match shader_stage {
        ReflectShaderStageFlags::COMPUTE => BindingShaderStage::COMPUTE,
        ReflectShaderStageFlags::VERTEX => BindingShaderStage::VERTEX,
        ReflectShaderStageFlags::FRAGMENT => BindingShaderStage::FRAGMENT,
        _ => panic!("Only one specified shader stage is supported."),
    };

    let name = name.to_string();

    if name == "Camera" {
        shader_stage = BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT;
    }

    BindingDescriptor {
        index: binding.binding,
        bind_type,
        name,
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
            signedness => panic!("unexpected signedness {}", signedness),
        }
    } else if type_description
        .type_flags
        .contains(ReflectTypeFlags::FLOAT)
    {
        NumberType::Float
    } else {
        panic!("unexpected type flag {:?}", type_description.type_flags);
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
            signedness => panic!("unexpected signedness {}", signedness),
        }
    } else if type_description
        .type_flags
        .contains(ReflectTypeFlags::FLOAT)
    {
        NumberType::Float
    } else {
        panic!("unexpected type flag {:?}", type_description.type_flags);
    };

    let width = traits.numeric.scalar.width;

    match (number_type, traits.numeric.vector.component_count, width) {
        (NumberType::UInt, 2, 8) => VertexFormat::Uchar2,
        (NumberType::UInt, 4, 8) => VertexFormat::Uchar4,
        (NumberType::Int, 2, 8) => VertexFormat::Char2,
        (NumberType::Int, 4, 8) => VertexFormat::Char4,
        (NumberType::UInt, 2, 16) => VertexFormat::Ushort2,
        (NumberType::UInt, 4, 16) => VertexFormat::Ushort4,
        (NumberType::Int, 2, 16) => VertexFormat::Short2,
        (NumberType::Int, 8, 16) => VertexFormat::Short4,
        (NumberType::Float, 2, 16) => VertexFormat::Half2,
        (NumberType::Float, 4, 16) => VertexFormat::Half4,
        (NumberType::Float, 0, 32) => VertexFormat::Float,
        (NumberType::Float, 2, 32) => VertexFormat::Float2,
        (NumberType::Float, 3, 32) => VertexFormat::Float3,
        (NumberType::Float, 4, 32) => VertexFormat::Float4,
        (NumberType::UInt, 0, 32) => VertexFormat::Uint,
        (NumberType::UInt, 2, 32) => VertexFormat::Uint2,
        (NumberType::UInt, 3, 32) => VertexFormat::Uint3,
        (NumberType::UInt, 4, 32) => VertexFormat::Uint4,
        (NumberType::Int, 0, 32) => VertexFormat::Int,
        (NumberType::Int, 2, 32) => VertexFormat::Int2,
        (NumberType::Int, 3, 32) => VertexFormat::Int3,
        (NumberType::Int, 4, 32) => VertexFormat::Int4,
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
            layout(set = 0, binding = 0) uniform Camera {
                mat4 ViewProj;
            };
            layout(set = 1, binding = 0) uniform texture2D Texture;

            void main() {
                v_Position = Vertex_Position;
                gl_Position = ViewProj * v_Position;
            }
        "#,
        )
        .get_spirv_shader(None);

        let layout = vertex_shader.reflect_layout(true).unwrap();
        assert_eq!(
            layout,
            ShaderLayout {
                entry_point: "main".into(),
                vertex_buffer_descriptors: vec![
                    VertexBufferDescriptor {
                        name: "Vertex".into(),
                        attributes: vec![
                            VertexAttributeDescriptor {
                                name: "Vertex_Position".into(),
                                format: VertexFormat::Float4,
                                offset: 0,
                                shader_location: 0,
                            },
                            VertexAttributeDescriptor {
                                name: "Vertex_Normal".into(),
                                format: VertexFormat::Uint4,
                                offset: 16,
                                shader_location: 1,
                            }
                        ],
                        step_mode: InputStepMode::Vertex,
                        stride: 32,
                    },
                    VertexBufferDescriptor {
                        name: "TestInstancing".into(),
                        attributes: vec![VertexAttributeDescriptor {
                            name: "I_TestInstancing_Property".into(),
                            format: VertexFormat::Uint4,
                            offset: 0,
                            shader_location: 2,
                        },],
                        step_mode: InputStepMode::Instance,
                        stride: 16,
                    }
                ],
                bind_groups: vec![
                    BindGroupDescriptor::new(
                        0,
                        vec![BindingDescriptor {
                            index: 0,
                            name: "Camera".into(),
                            bind_type: BindType::Uniform {
                                dynamic: false,
                                property: UniformProperty::Struct(vec![UniformProperty::Mat4]),
                            },
                            shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
                        }]
                    ),
                    BindGroupDescriptor::new(
                        1,
                        vec![BindingDescriptor {
                            index: 0,
                            name: "Texture".into(),
                            bind_type: BindType::SampledTexture {
                                multisampled: false,
                                dimension: TextureViewDimension::D2,
                                component_type: TextureComponentType::Float,
                            },
                            shader_stage: BindingShaderStage::VERTEX,
                        }]
                    ),
                ]
            }
        );
    }

    #[test]
    #[should_panic(expected = "Vertex attribute buffer names must be consecutive.")]
    fn test_reflection_consecutive_buffer_validation() {
        let vertex_shader = Shader::from_glsl(
            ShaderStage::Vertex,
            r#"
            #version 450
            layout(location = 0) in vec4 Vertex_Position;
            layout(location = 1) in uvec4 Other_Property;
            layout(location = 2) in uvec4 Vertex_Normal;

            layout(location = 0) out vec4 v_Position;
            layout(set = 0, binding = 0) uniform Camera {
                mat4 ViewProj;
            };
            layout(set = 1, binding = 0) uniform texture2D Texture;

            void main() {
                v_Position = Vertex_Position;
                gl_Position = ViewProj * v_Position;
            }
        "#,
        )
        .get_spirv_shader(None);

        let _layout = vertex_shader.reflect_layout(true).unwrap();
    }
}
