use std::convert::TryInto;

use bevy_core::cast_slice;
use bevy_utils::HashMap;
use naga::{ScalarKind, StorageClass, VectorSize};

use crate::{
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, BindingShaderStage, InputStepMode,
        UniformProperty, VertexAttribute, VertexBufferLayout, VertexFormat,
    },
    shader::ShaderLayout,
    texture::{StorageTextureAccess, TextureFormat, TextureSampleType, TextureViewDimension},
};

impl ShaderLayout {
    pub fn from_spirv(spirv_data: &[u32], bevy_conventions: bool) -> ShaderLayout {
        let options = naga::front::spv::Options::default();
        let module = naga::front::spv::parse_u8_slice(cast_slice(spirv_data), &options)
            .expect("failed to parse");

        assert!(module.entry_points.len() == 1, "expected one entry point");
        let entry_point = &module.entry_points[0];
        let vertex_buffer_layout =
            reflect_vertex_buffer_layout(&entry_point.function, &module, bevy_conventions);

        let shader_stage = match entry_point.stage {
            naga::ShaderStage::Vertex => BindingShaderStage::VERTEX,
            naga::ShaderStage::Fragment => BindingShaderStage::FRAGMENT,
            naga::ShaderStage::Compute => BindingShaderStage::COMPUTE,
        };
        let bind_groups = reflect_bind_groups(&module, shader_stage);

        ShaderLayout {
            bind_groups,
            vertex_buffer_layout,
            entry_point: entry_point.name.clone(),
        }
    }
}

fn reflect_vertex_buffer_layout(
    function: &naga::Function,
    module: &naga::Module,
    bevy_conventions: bool,
) -> Vec<VertexBufferLayout> {
    let mut vertex_attribute_layouts: Vec<_> = function
        .arguments
        .iter()
        .filter_map(|argument| {
            let location = match argument.binding.as_ref()? {
                naga::Binding::Location { location, .. } => *location,
                _ => return None,
            };
            let name = argument
                .name
                .clone()
                .expect("expected vertex attribute to have a name");

            let current_buffer_name = if bevy_conventions {
                name.clone()
            } else {
                "DefaultVertex".to_string()
            };
            let step_mode = if bevy_conventions && name.starts_with("I_") {
                InputStepMode::Instance
            } else {
                InputStepMode::Vertex
            };

            let ty = module
                .types
                .try_get(argument.ty)
                .expect("vertex attribute references inexistent type");
            let attribute = VertexAttribute {
                name: name.into(),
                format: reflect_vertex_format(&ty.inner),
                offset: 0,
                shader_location: location,
            };
            let layout = VertexBufferLayout {
                name: current_buffer_name.into(),
                stride: 0,
                step_mode,
                attributes: vec![attribute],
            };
            Some(layout)
        })
        .collect();

    vertex_attribute_layouts.sort_by_key(|layout| layout.attributes[0].shader_location);

    vertex_attribute_layouts
}

fn reflect_vertex_format(type_description: &naga::TypeInner) -> VertexFormat {
    match type_description {
        naga::TypeInner::Scalar { kind, width } => match (kind, width) {
            (ScalarKind::Float, 4) => VertexFormat::Float32,
            (ScalarKind::Sint, 4) => VertexFormat::Sint32,
            (ScalarKind::Uint, 4) => VertexFormat::Uint32,
            (ScalarKind::Bool, _) => panic!("bool vertex format not supported"),
            (kind, width) => panic!("unexpected vertex format: {:?}{}", kind, width),
        },
        naga::TypeInner::Vector { size, kind, width } => match (kind, size, width) {
            (ScalarKind::Float, VectorSize::Bi, 2) => VertexFormat::Float16x2,
            (ScalarKind::Float, VectorSize::Quad, 2) => VertexFormat::Float16x4,
            (ScalarKind::Float, VectorSize::Bi, 4) => VertexFormat::Float32x2,
            (ScalarKind::Float, VectorSize::Tri, 4) => VertexFormat::Float32x3,
            (ScalarKind::Float, VectorSize::Quad, 4) => VertexFormat::Float32x4,
            (ScalarKind::Sint, VectorSize::Bi, 8) => VertexFormat::Sint8x2,
            (ScalarKind::Sint, VectorSize::Quad, 8) => VertexFormat::Sint8x4,
            (ScalarKind::Sint, VectorSize::Bi, 2) => VertexFormat::Sint16x2,
            (ScalarKind::Sint, VectorSize::Quad, 2) => VertexFormat::Sint16x4,
            (ScalarKind::Sint, VectorSize::Bi, 4) => VertexFormat::Sint32x2,
            (ScalarKind::Sint, VectorSize::Tri, 4) => VertexFormat::Sint32x3,
            (ScalarKind::Sint, VectorSize::Quad, 4) => VertexFormat::Sint32x4,
            (ScalarKind::Uint, VectorSize::Bi, 2) => VertexFormat::Uint16x2,
            (ScalarKind::Uint, VectorSize::Quad, 2) => VertexFormat::Uint16x4,
            (ScalarKind::Uint, VectorSize::Bi, 4) => VertexFormat::Uint32x2,
            (ScalarKind::Uint, VectorSize::Tri, 4) => VertexFormat::Uint32x3,
            (ScalarKind::Uint, VectorSize::Quad, 4) => VertexFormat::Uint32x4,
            (ScalarKind::Bool, _, _) => panic!("bool vector vertex format not supported"),
            (kind, size, width) => {
                panic!(
                    "expected vertex format vector: {:?}{}x{}",
                    kind, width, *size as u8
                )
            }
        },
        naga::TypeInner::Matrix {
            columns,
            rows,
            width: _,
        } => panic!(
            "matrix vertex format {}x{} not supported",
            *columns as u8, *rows as u8
        ),
        other => panic!("unexpected vertex format {:?}", other),
    }
}

fn reflect_bind_groups(
    module: &naga::Module,
    shader_stage: BindingShaderStage,
) -> Vec<BindGroupDescriptor> {
    let binding_descriptors = module
        .global_variables
        .iter()
        .map(|(_, variable)| variable)
        .filter(|variable| {
            matches!(
                variable.class,
                StorageClass::Uniform | StorageClass::Storage | StorageClass::Handle
            )
        })
        .filter_map(|variable| {
            let binding = variable.binding.as_ref()?;
            Some((variable, binding))
        })
        .map(|(variable, binding)| {
            let ty = module
                .types
                .try_get(variable.ty)
                .expect("resource binding references inexistent type");

            let name = match variable.name.clone() {
                Some(name) if !name.is_empty() => name,
                _ => ty.name.clone().unwrap_or_default(),
            };

            let bind_type = reflect_bind_type(&module, variable, &ty.inner);
            let binding_descriptor = BindingDescriptor {
                index: binding.binding,
                bind_type,
                name,
                shader_stage,
            };
            (binding, binding_descriptor)
        });

    let mut bind_groups = HashMap::<u32, Vec<_>>::default();
    for (binding, binding_descriptor) in binding_descriptors {
        bind_groups
            .entry(binding.group)
            .or_default()
            .push(binding_descriptor);
    }

    let mut groups: Vec<_> = bind_groups
        .into_iter()
        .map(|(index, bindings)| BindGroupDescriptor::new(index, bindings))
        .collect();

    for group in &mut groups {
        group.bindings.sort_by_key(|binding| binding.index);
    }
    groups.sort_by_key(|bind_group| bind_group.index);

    groups
}

fn reflect_bind_type(
    module: &naga::Module,
    variable: &naga::GlobalVariable,
    ty: &naga::TypeInner,
) -> BindType {
    match variable.class {
        naga::StorageClass::Uniform => BindType::Uniform {
            has_dynamic_offset: false,
            property: reflect_uniform(module, ty),
        },
        naga::StorageClass::Handle => match *ty {
            naga::TypeInner::Image {
                dim,
                arrayed,
                class,
            } => {
                let view_dimension = reflect_dimension(dim, arrayed);
                match class {
                    naga::ImageClass::Sampled { kind, multi } => BindType::Texture {
                        multisampled: multi,
                        view_dimension,
                        sample_type: match kind {
                            ScalarKind::Sint => TextureSampleType::Sint,
                            ScalarKind::Uint => TextureSampleType::Uint,
                            ScalarKind::Float => TextureSampleType::Float { filterable: true },
                            ScalarKind::Bool => panic!("invalid texture sample type: `bool`"),
                        },
                    },
                    naga::ImageClass::Depth => BindType::Texture {
                        multisampled: false,
                        view_dimension,
                        sample_type: TextureSampleType::Depth,
                    },
                    naga::ImageClass::Storage(format) => BindType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: reflect_texture_format(format),
                        view_dimension,
                    },
                }
            }
            naga::TypeInner::Sampler { comparison } => BindType::Sampler {
                comparison,
                filtering: true,
            },

            naga::TypeInner::Array { base, size, .. } => match size {
                naga::ArraySize::Constant(size) => {
                    let inner_ty = module.types.try_get(base).unwrap();
                    let len = get_constant_usize(module, size)
                        .expect("expected integer constant for array length");
                    panic!(
                        "unsupported binding: sampler array [{:?}; {}]",
                        inner_ty.inner, len
                    );
                }
                naga::ArraySize::Dynamic => {
                    panic!("unsupported binding: handle array with dynamic size")
                }
            },

            ref other => panic!(
                "handle storage class not with image or sampler type: {:?}",
                other
            ),
        },
        naga::StorageClass::Storage => BindType::StorageBuffer {
            has_dynamic_offset: false,
            readonly: !variable.storage_access.contains(naga::StorageAccess::STORE),
        },
        other => panic!("unexpected storage type for shader binding: {:?}", other),
    }
}

fn reflect_texture_format(format: naga::StorageFormat) -> TextureFormat {
    match format {
        naga::StorageFormat::R8Unorm => TextureFormat::R8Unorm,
        naga::StorageFormat::R8Snorm => TextureFormat::R8Snorm,
        naga::StorageFormat::R8Uint => TextureFormat::R8Uint,
        naga::StorageFormat::R8Sint => TextureFormat::R8Sint,
        naga::StorageFormat::R16Uint => TextureFormat::R16Uint,
        naga::StorageFormat::R16Sint => TextureFormat::R16Sint,
        naga::StorageFormat::R16Float => TextureFormat::R16Float,
        naga::StorageFormat::Rg8Unorm => TextureFormat::Rg8Unorm,
        naga::StorageFormat::Rg8Snorm => TextureFormat::Rg8Snorm,
        naga::StorageFormat::Rg8Uint => TextureFormat::Rg8Uint,
        naga::StorageFormat::Rg8Sint => TextureFormat::Rg8Sint,
        naga::StorageFormat::R32Uint => TextureFormat::R32Uint,
        naga::StorageFormat::R32Sint => TextureFormat::R32Sint,
        naga::StorageFormat::R32Float => TextureFormat::R32Float,
        naga::StorageFormat::Rg16Uint => TextureFormat::Rg16Uint,
        naga::StorageFormat::Rg16Sint => TextureFormat::Rg16Sint,
        naga::StorageFormat::Rg16Float => TextureFormat::Rg16Float,
        naga::StorageFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
        naga::StorageFormat::Rgba8Snorm => TextureFormat::Rgba8Snorm,
        naga::StorageFormat::Rgba8Uint => TextureFormat::Rgba8Uint,
        naga::StorageFormat::Rgba8Sint => TextureFormat::Rgba8Sint,
        naga::StorageFormat::Rgb10a2Unorm => TextureFormat::Rgb10a2Unorm,
        naga::StorageFormat::Rg11b10Float => TextureFormat::Rg11b10Float,
        naga::StorageFormat::Rg32Uint => TextureFormat::Rg32Uint,
        naga::StorageFormat::Rg32Sint => TextureFormat::Rg32Sint,
        naga::StorageFormat::Rg32Float => TextureFormat::Rg32Float,
        naga::StorageFormat::Rgba16Uint => TextureFormat::Rgba16Uint,
        naga::StorageFormat::Rgba16Sint => TextureFormat::Rgba16Sint,
        naga::StorageFormat::Rgba16Float => TextureFormat::Rgba16Float,
        naga::StorageFormat::Rgba32Uint => TextureFormat::Rgba32Uint,
        naga::StorageFormat::Rgba32Sint => TextureFormat::Rgba32Sint,
        naga::StorageFormat::Rgba32Float => TextureFormat::Rgba32Float,
    }
}

fn reflect_uniform(module: &naga::Module, ty: &naga::TypeInner) -> UniformProperty {
    match ty {
        naga::TypeInner::Struct { members, .. } => {
            let members = members
                .iter()
                .map(|member| {
                    let ty = module.types.try_get(member.ty).unwrap();
                    reflect_uniform(module, &ty.inner)
                })
                .collect();
            UniformProperty::Struct(members)
        }

        naga::TypeInner::Scalar { kind, .. } => match kind {
            naga::ScalarKind::Float => UniformProperty::Float,
            naga::ScalarKind::Uint => UniformProperty::UInt,
            naga::ScalarKind::Sint => UniformProperty::Int,
            naga::ScalarKind::Bool => {
                panic!("unsupported uniform property: {:?}", naga::ScalarKind::Bool)
            }
        },
        naga::TypeInner::Vector { size, kind, .. } => match (kind, size) {
            (ScalarKind::Sint, VectorSize::Bi) => UniformProperty::IVec2,
            (ScalarKind::Uint, VectorSize::Quad) => UniformProperty::UVec4,
            (ScalarKind::Float, VectorSize::Bi) => UniformProperty::Vec2,
            (ScalarKind::Float, VectorSize::Tri) => UniformProperty::Vec3,
            (ScalarKind::Float, VectorSize::Quad) => UniformProperty::Vec4,
            (ScalarKind::Bool, size) => panic!(
                "unsupported uniform property: {:?}x{}",
                naga::ScalarKind::Bool,
                *size as u8
            ),
            (kind, size) => panic!("unsupported uniform property: {:?}x{}", kind, *size as u8),
        },
        naga::TypeInner::Matrix { columns, rows, .. } => match (columns, rows) {
            (VectorSize::Tri, VectorSize::Tri) => UniformProperty::Mat3,
            (VectorSize::Quad, VectorSize::Quad) => UniformProperty::Mat4,
            (columns, rows) => panic!(
                "unsupported uniform property: {}x{} matrix",
                *columns as u8, *rows as u8
            ),
        },
        naga::TypeInner::Array { base, size, .. } => match size {
            naga::ArraySize::Constant(size) => {
                let inner_ty = module.types.try_get(*base).unwrap();
                let inner = reflect_uniform(module, &inner_ty.inner);
                let len = get_constant_usize(module, *size)
                    .expect("expected integer constant for array length");

                UniformProperty::Array(Box::new(inner), len)
            }
            naga::ArraySize::Dynamic => panic!("unsupported uniform property: dynamic array size"),
        },
        other => panic!("unsupported uniform property: {:?}", other),
    }
}

fn reflect_dimension(dim: naga::ImageDimension, arrayed: bool) -> TextureViewDimension {
    match (dim, arrayed) {
        (naga::ImageDimension::D1, false) => TextureViewDimension::D1,
        (naga::ImageDimension::D2, false) => TextureViewDimension::D2,
        (naga::ImageDimension::D3, false) => TextureViewDimension::D3,
        (naga::ImageDimension::Cube, false) => TextureViewDimension::Cube,
        (naga::ImageDimension::D2, true) => TextureViewDimension::D2Array,
        (naga::ImageDimension::Cube, true) => TextureViewDimension::CubeArray,
        (other, true) => panic!("invalid image type: {:?} array", other),
    }
}

fn get_constant_usize(
    module: &naga::Module,
    constant: naga::Handle<naga::Constant>,
) -> Option<usize> {
    let constant = module.constants.try_get(constant)?;
    match (constant.specialization, &constant.inner) {
        (Some(spec), _) => Some(spec as usize),
        (None, naga::ConstantInner::Composite { .. }) => None,
        (None, naga::ConstantInner::Scalar { value, .. }) => match *value {
            naga::ScalarValue::Sint(int) => int.try_into().ok(),
            naga::ScalarValue::Uint(int) => int.try_into().ok(),
            naga::ScalarValue::Float(_) | naga::ScalarValue::Bool(_) => None,
        },
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

            layout(set = 2, binding = 0) uniform texture2D ColorMaterial_texture;
            layout(set = 2, binding = 1) uniform sampler ColorMaterial_texture_sampler;
            layout(set = 2, binding = 2) uniform samplerCubeArray arrayTextureSampler;
            layout(set = 2, binding = 3) buffer TextureAtlas_textures { float data; };

            void main() {
                v_Position = Vertex_Position;
                gl_Position = ViewProj * v_Position;
            }
        "#,
        )
        .get_spirv_shader(None)
        .unwrap();

        let layout = vertex_shader.reflect_layout(true).unwrap();
        pretty_assertions::assert_eq!(
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
                    BindGroupDescriptor::new(
                        2,
                        vec![
                            BindingDescriptor {
                                index: 0,
                                name: "ColorMaterial_texture".into(),
                                bind_type: BindType::Texture {
                                    multisampled: false,
                                    view_dimension: TextureViewDimension::D2,
                                    sample_type: TextureSampleType::Float { filterable: true }
                                },
                                shader_stage: BindingShaderStage::VERTEX,
                            },
                            BindingDescriptor {
                                index: 1,
                                name: "ColorMaterial_texture_sampler".into(),
                                bind_type: BindType::Sampler {
                                    filtering: true,
                                    comparison: false,
                                },
                                shader_stage: BindingShaderStage::VERTEX,
                            },
                            BindingDescriptor {
                                index: 2,
                                name: "arrayTextureSampler".into(),
                                bind_type: BindType::Texture {
                                    multisampled: false,
                                    view_dimension: TextureViewDimension::CubeArray,
                                    sample_type: TextureSampleType::Float { filterable: true },
                                },
                                shader_stage: BindingShaderStage::VERTEX,
                            },
                            BindingDescriptor {
                                index: 3,
                                name: "TextureAtlas_textures".into(),
                                bind_type: BindType::StorageBuffer {
                                    has_dynamic_offset: false,
                                    readonly: false,
                                },
                                shader_stage: BindingShaderStage::VERTEX,
                            },
                        ]
                    )
                ]
            }
        );
    }
}
