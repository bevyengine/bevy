use crate::{
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, InputStepMode, UniformProperty,
        VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat, BindingShaderStage,
    },
    texture::{TextureComponentType, TextureViewDimension},
};
use smallvec::SmallVec;
use bevy_core::AsBytes;

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
        let module = naga::front::spv::parse_u8_slice(spirv_data.as_bytes())
            .expect("Failed to reflect shader layout");

        // Right now, we only support a single entry-point per shader.
        // TODO: Change this down the road

        assert_eq!(module.entry_points.len(), 1, "shaders with multiple entry points are not supported yet");

        let entry_point = &module.entry_points[0];

        let mut bind_groups = Vec::new();
        let mut vertex_buffer_descriptors = Vec::new();

        for (_handle, global) in module.global_variables.iter() {
            if let Some(binding) = &global.binding {
                match binding {
                    &naga::Binding::Descriptor { set, binding } => {
                        let bindings = if let Some(bind_group) = bind_groups
                            .iter_mut()
                            .find(|bind_group: &&mut BindGroupDescriptor| bind_group.index == set) {
                                &mut bind_group.bindings
                            } else {
                                bind_groups.push(BindGroupDescriptor::new(set, vec![]));
                                &mut bind_groups.last_mut().unwrap().bindings
                            };
                            
                        bindings.push(reflect_binding_descriptor(
                            &module,
                            global,
                            binding,
                            entry_point.stage
                        ));
                    }
                    &naga::Binding::Location(shader_location) if global.class == naga::StorageClass::Input => {
                        let (buffer_name, step_mode) = if bevy_conventions {
                            let name = global.name.as_ref().unwrap();
                            let split: SmallVec<[_; 3]> = name.split('_').collect();

                            match &split[..] {
                                &["I", buffer_name, _] => {
                                    (buffer_name, InputStepMode::Instance)
                                }
                                &[buffer_name, _] => {
                                    (buffer_name, InputStepMode::Vertex)
                                }
                                _ => panic!("Vertex attributes must follow the form (I_)BUFFERNAME_PROPERTYNAME. For example: Vertex_Position or I_TestInstancing_Property"),
                            }
                        } else {
                            ("DefaultVertex", InputStepMode::Vertex)
                        };

                        let buffer_desc = if let Some(buffer_desc) = vertex_buffer_descriptors
                            .iter_mut()
                            .find(|buffer_desc: &&mut VertexBufferDescriptor| buffer_desc.name.as_ref() == buffer_name)
                        {
                            buffer_desc
                        } else {
                            vertex_buffer_descriptors.push(VertexBufferDescriptor {
                                name: buffer_name.to_owned().into(),
                                stride: 0, // to be filled in later on
                                step_mode,
                                attributes: vec![],
                            });
                            vertex_buffer_descriptors.last_mut().unwrap()
                        };

                        buffer_desc.attributes.push(reflect_vertex_attribute_desc(&module, global, shader_location));
                    }
                    _ => {},
                }
            }
        }

        // Sort the bind groups and attributes by set, binding, and location.
        bind_groups.sort_unstable_by_key(|desc| desc.index);
        
        for binding_desc in bind_groups.iter_mut().map(|desc| &mut desc.bindings[..]) {
            binding_desc.sort_unstable_by_key(|desc| desc.index);
        }

        for buf_desc in vertex_buffer_descriptors.iter_mut() {
            buf_desc.attributes.sort_unstable_by_key(|desc| desc.shader_location);

            // Accumulate offsets and stride.
            buf_desc.stride = buf_desc.attributes.iter_mut().fold(0, |offset, attr_desc| {
                attr_desc.offset = offset;
                offset + attr_desc.format.get_size()
            });
        }

        ShaderLayout {
            bind_groups,
            vertex_buffer_descriptors,
            entry_point: entry_point.name.clone(),
        }
    }
}

fn reflect_vertex_attribute_desc(module: &naga::Module, global: &naga::GlobalVariable, shader_location: u32) -> VertexAttributeDescriptor {
    use naga::{TypeInner::*, ScalarKind::*, VectorSize::*};

    let ty = &module.types[global.ty];

    let format = match ty.inner {
        Scalar { kind, width } => match (kind, width) {
            (Uint, 4) => VertexFormat::Uint,
            (Sint, 4) =>  VertexFormat::Int,
            (Float, 4) => VertexFormat::Float,
            _ => unimplemented!(),
        },
        Vector { size, kind, width } => match (size, kind, width) {
            (Bi, Uint, 1) => VertexFormat::Uchar2,
            (Bi, Sint, 1) => VertexFormat::Char2,
            (Bi, Uint, 2) => VertexFormat::Ushort2,
            (Bi, Sint, 2) => VertexFormat::Short2,
            (Bi, Float, 2) => VertexFormat::Half2,
            (Bi, Uint, 4) => VertexFormat::Uint2,
            (Bi, Sint, 4) => VertexFormat::Int2,
            (Bi, Float, 4) => VertexFormat::Float2,

            (Tri, Uint, 4) => VertexFormat::Uint3,
            (Tri, Sint, 4) => VertexFormat::Int3,
            (Tri, Float, 4) => VertexFormat::Float3,

            (Quad, Uint, 1) => VertexFormat::Uchar4,
            (Quad, Sint, 1) => VertexFormat::Char4,
            (Quad, Uint, 2) => VertexFormat::Ushort4,
            (Quad, Sint, 2) => VertexFormat::Short4,
            (Quad, Float, 2) => VertexFormat::Half4,
            (Quad, Uint, 4) => VertexFormat::Uint4,
            (Quad, Sint, 4) => VertexFormat::Int4,
            (Quad, Float, 4) => VertexFormat::Float4,
            _ => unimplemented!(),
        }
        _ => unimplemented!()
    };

    VertexAttributeDescriptor {
        name: global.name.as_ref().unwrap().to_owned().into(),
        offset: 0, // too be filled in later
        format,
        shader_location,
    }
}

fn reflect_binding_descriptor(module: &naga::Module, global: &naga::GlobalVariable, binding: u32, shader_stage: naga::ShaderStage) -> BindingDescriptor {
    let (name, bind_type) = {
        let ty = &module.types[global.ty];
        match global.class {
            naga::StorageClass::Uniform => (
                ty.name.as_ref().unwrap().clone(),
                BindType::Uniform {
                    dynamic: false,
                    properties: vec![reflect_uniform_type(&module, &module.types[global.ty])],
                }
            ),
            naga::StorageClass::StorageBuffer => (
                ty.name.as_ref().unwrap().clone(),
                BindType::StorageBuffer {
                    dynamic: false,
                    readonly: true,
                }
            ),
            _ => {
                let bind_type = match ty.inner {
                    naga::TypeInner::Image { base, dim, flags } => {
                        assert!(flags.contains(naga::ImageFlags::SAMPLED), "image must be sampled");

                        let component_type = match &module.types[base].inner {
                            naga::TypeInner::Scalar { kind, width: 4 } => match kind {
                                naga::ScalarKind::Sint => TextureComponentType::Sint,
                                naga::ScalarKind::Uint => TextureComponentType::Uint,
                                naga::ScalarKind::Float => TextureComponentType::Float,
                                naga::ScalarKind::Bool => unimplemented!(),
                            },
                            _ => unimplemented!(),
                        };

                        BindType::SampledTexture {
                            dimension: match dim {
                                naga::ImageDimension::D1 => TextureViewDimension::D1,
                                naga::ImageDimension::D2 => TextureViewDimension::D2,
                                naga::ImageDimension::D3 => TextureViewDimension::D3,
                                naga::ImageDimension::Cube => TextureViewDimension::Cube,
                            },
                            component_type,
                            multisampled: flags.contains(naga::ImageFlags::MULTISAMPLED),
                        }
                    }
                    naga::TypeInner::Sampler { comparison } => BindType::Sampler { comparison },
                    _ => unimplemented!("unsupported bind type: {:?}", ty),
                };

                (global.name.as_ref().unwrap().clone(), bind_type)
            }
        }
    };

    BindingDescriptor {
        name,
        index: binding,
        bind_type,
        shader_stage: match shader_stage {
            naga::ShaderStage::Vertex => BindingShaderStage::VERTEX,
            naga::ShaderStage::Fragment => BindingShaderStage::FRAGMENT,
            naga::ShaderStage::Compute => BindingShaderStage::COMPUTE,
        },
    }
}

fn reflect_uniform_type(module: &naga::Module, ty: &naga::Type) -> UniformProperty {
    use naga::{TypeInner, ScalarKind, VectorSize};
    
    if let Some(prop) = match &ty.inner {
        TypeInner::Scalar { kind, width: 4 } => {
            match kind {
                ScalarKind::Sint => Some(UniformProperty::Int),
                ScalarKind::Uint => Some(UniformProperty::UInt),
                ScalarKind::Float => Some(UniformProperty::Float),
                ScalarKind::Bool => None,
            }
        }
        TypeInner::Vector { size, kind, width } => {
            match (size, kind, width) {
                (VectorSize::Bi, ScalarKind::Sint, 4) => Some(UniformProperty::IVec2),
                (VectorSize::Bi, ScalarKind::Float, 4) => Some(UniformProperty::Vec2),
                (VectorSize::Tri, ScalarKind::Float, 4) => Some(UniformProperty::Vec3),
                (VectorSize::Quad, ScalarKind::Uint, 4) => Some(UniformProperty::UVec4),
                (VectorSize::Quad, ScalarKind::Float, 4) => Some(UniformProperty::Vec4),
                _ => None,
            }
        }
        TypeInner::Matrix { columns, rows, kind, width } => {
            match (columns, rows, kind, width) {
                (VectorSize::Tri, VectorSize::Tri, ScalarKind::Float, 4) => Some(UniformProperty::Mat3),
                (VectorSize::Quad, VectorSize::Quad, ScalarKind::Float, 4) => Some(UniformProperty::Mat4),
                _ => None,
            }
        }
        TypeInner::Struct { members } => Some(UniformProperty::Struct(
            members.iter().map(|member| {
                reflect_uniform_type(module, &module.types[member.ty])
            }).collect()
        )),
        _ => None,
    } { prop } else { panic!("unexpected uniform property format: {:?}", ty.inner) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::{Shader, ShaderStage};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_reflection_compare() {
        let vertex_shader = Shader::from_glsl(
            ShaderStage::Vertex,
            r#"
            #version 440 // TODO: until we're using naga to compile from glsl to spirv, keep this as 440, not 450
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
                                properties: vec![UniformProperty::Struct(vec![
                                    UniformProperty::Mat4
                                ])],
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
                            shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
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
            #version 440 // TODO: until we're using naga to compile from glsl to spirv, keep this as 440, not 450
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
