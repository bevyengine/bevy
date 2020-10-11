use super::{Gl, WebGl2RenderingContext, WebGlProgram, WebGlShader};
use crate::{gl_call, GlVertexFormat};
use bevy_render::{
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, BindingShaderStage, InputStepMode,
        PipelineLayout, UniformProperty, VertexAttributeDescriptor, VertexBufferDescriptor,
        VertexFormat,
    },
    texture::{TextureComponentType, TextureViewDimension},
};

use web_sys::WebGlActiveInfo;

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl_call!(context.create_shader(shader_type))
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    gl_call!(context.shader_source(&shader, source));
    gl_call!(context.compile_shader(&shader));

    if gl_call!(context.get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS))
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    shaders: &[WebGlShader],
) -> Result<WebGlProgram, String> {
    let program = gl_call!(context.create_program())
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    for shader in shaders {
        log::info!("attaching shader");
        gl_call!(context.attach_shader(&program, shader));
    }
    gl_call!(context.link_program(&program));

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

fn get_vertex_format(gl_type: u32) -> VertexFormat {
    match gl_type {
        Gl::FLOAT => VertexFormat::Float,
        Gl::FLOAT_VEC2 => VertexFormat::Float2,
        Gl::FLOAT_VEC3 => VertexFormat::Float3,
        Gl::FLOAT_VEC4 => VertexFormat::Float4,
        Gl::INT => VertexFormat::Int,
        _ => panic!("unknown vertex attribute type: {:?}", gl_type),
    }
}

pub fn reflect_layout(context: &WebGl2RenderingContext, program: &WebGlProgram) -> PipelineLayout {
    let gl = context;
    let mut attributes = vec![];
    let mut offset = 0;
    let mut shader_location = 0;

    let active_attributes = gl
        .get_program_parameter(&program, Gl::ACTIVE_ATTRIBUTES)
        .as_f64()
        .unwrap() as u32;
    log::info!("active attributes: {:?}", active_attributes);
    for index in 0..active_attributes {
        let info: WebGlActiveInfo = gl.get_active_attrib(&program, index).unwrap();
        let name = info.name();
        log::info!(
            "index {:?}: name: {:?} type: {:?}, size: {:?}",
            index,
            name,
            info.type_(),
            info.size()
        );
        if name == "gl_VertexID" {
            continue;
        }

        let format = get_vertex_format(info.type_());
        let size = format.get_size();
        attributes.push(VertexAttributeDescriptor {
            name: info.name().into(),
            offset,
            format,
            shader_location,
        });
        offset += size;
        shader_location += 1;
    }

    let vertex_buffer_descriptors = vec![VertexBufferDescriptor {
        name: "Vertex".into(),
        stride: offset,
        step_mode: InputStepMode::Vertex,
        attributes,
    }];
    let mut bind_groups = vec![];

    let active_uniform_blocks = gl
        .get_program_parameter(&program, Gl::ACTIVE_UNIFORM_BLOCKS)
        .as_f64()
        .unwrap() as u32;
    log::info!("active uniform blocks: {:?}", active_uniform_blocks);

    bind_groups.push(BindGroupDescriptor::new(
        0,
        vec![BindingDescriptor {
            name: "Camera".to_string(),
            index: 0,
            bind_type: BindType::Uniform {
                dynamic: false,
                property: UniformProperty::Struct(vec![UniformProperty::Mat4]),
            },
            shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
        }],
    ));

    let mut index = 1;
    for uniform_index in 0..active_uniform_blocks {
        let name = gl
            .get_active_uniform_block_name(&program, uniform_index)
            .unwrap();
        if name == "Camera" {
            continue;
        }
        let size = gl
            .get_active_uniform_block_parameter(
                &program,
                uniform_index,
                Gl::UNIFORM_BLOCK_DATA_SIZE,
            )
            .unwrap()
            .as_f64()
            .unwrap() as u32;
        let active_uniforms = gl
            .get_active_uniform_block_parameter(
                &program,
                uniform_index,
                Gl::UNIFORM_BLOCK_ACTIVE_UNIFORMS,
            )
            .unwrap()
            .as_f64()
            .unwrap() as u32;
        let active_uniform_indices = gl
            .get_active_uniform_block_parameter(
                &program,
                uniform_index,
                Gl::UNIFORM_BLOCK_ACTIVE_UNIFORM_INDICES,
            )
            .unwrap();

        log::info!(
            "index: {:?}, name: {:?} size: {:?} active_uniforms: {:?} indices: {:?}",
            uniform_index,
            name,
            size,
            active_uniforms,
            active_uniform_indices
        );
        let property = UniformProperty::Array(Box::new(UniformProperty::UInt), size as usize / 4);
        let bindings = vec![BindingDescriptor {
            name,
            index: 0,
            bind_type: BindType::Uniform {
                dynamic: false,
                property,
            },
            shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
        }];
        bind_groups.push(BindGroupDescriptor::new(index, bindings));
        index += 1;
    }

    let active_uniforms = gl
        .get_program_parameter(&program, Gl::ACTIVE_UNIFORMS)
        .as_f64()
        .unwrap() as u32;
    log::info!("active uniforms: {:?}", active_uniforms);
    for uniform_index in 0..active_uniforms {
        let info = gl.get_active_uniform(&program, uniform_index).unwrap();
        if info.type_() == Gl::SAMPLER_2D {
            log::info!(
                "index {:?}: name: {:?} type: {:?}, size: {:?}",
                uniform_index,
                info.name(),
                info.type_(),
                info.size(),
            );
            let bindings = vec![BindingDescriptor {
                name: info.name(),
                index: 0,
                bind_type: BindType::SampledTexture {
                    multisampled: false,
                    dimension: TextureViewDimension::D2,
                    component_type: TextureComponentType::Float,
                },
                shader_stage: BindingShaderStage::FRAGMENT,
            }];
            bind_groups.push(BindGroupDescriptor::new(index, bindings));
            index += 1;
        }
    }

    PipelineLayout {
        bind_groups,
        vertex_buffer_descriptors,
    }
}

pub fn gl_vertex_format(vertex_format: &VertexFormat) -> GlVertexFormat {
    let (format, nr_of_components, normalized) = match vertex_format {
        VertexFormat::Uchar2 => (Gl::BYTE, 2, false),
        VertexFormat::Uchar4 => (Gl::BYTE, 4, false),
        VertexFormat::Char2 => (Gl::BYTE, 2, false),
        VertexFormat::Char4 => (Gl::BYTE, 4, false),
        VertexFormat::Uchar2Norm => (Gl::BYTE, 2, true),
        VertexFormat::Uchar4Norm => (Gl::BYTE, 4, true),
        VertexFormat::Char2Norm => (Gl::BYTE, 2, true),
        VertexFormat::Char4Norm => (Gl::BYTE, 4, true),
        VertexFormat::Ushort2 => (Gl::UNSIGNED_SHORT, 2, false),
        VertexFormat::Ushort4 => (Gl::UNSIGNED_SHORT, 4, false),
        VertexFormat::Short2 => (Gl::SHORT, 2, false),
        VertexFormat::Short4 => (Gl::SHORT, 4, false),
        VertexFormat::Ushort2Norm => (Gl::UNSIGNED_SHORT, 2, true),
        VertexFormat::Ushort4Norm => (Gl::UNSIGNED_SHORT, 4, true),
        VertexFormat::Short2Norm => (Gl::SHORT, 2, true),
        VertexFormat::Short4Norm => (Gl::SHORT, 4, true),
        VertexFormat::Half2 => (Gl::HALF_FLOAT, 2, false),
        VertexFormat::Half4 => (Gl::HALF_FLOAT, 4, false),
        VertexFormat::Float => (Gl::FLOAT, 1, false),
        VertexFormat::Float2 => (Gl::FLOAT, 2, false),
        VertexFormat::Float3 => (Gl::FLOAT, 3, false),
        VertexFormat::Float4 => (Gl::FLOAT, 4, false),
        VertexFormat::Uint => (Gl::UNSIGNED_INT, 1, false),
        VertexFormat::Uint2 => (Gl::UNSIGNED_INT, 2, false),
        VertexFormat::Uint3 => (Gl::UNSIGNED_INT, 3, false),
        VertexFormat::Uint4 => (Gl::UNSIGNED_INT, 4, false),
        VertexFormat::Int => (Gl::INT, 1, false),
        VertexFormat::Int2 => (Gl::INT, 2, false),
        VertexFormat::Int3 => (Gl::INT, 3, false),
        VertexFormat::Int4 => (Gl::INT, 4, false),
    };
    GlVertexFormat {
        format,
        nr_of_components,
        normalized,
    }
}
