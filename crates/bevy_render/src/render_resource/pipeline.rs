use crate::render_resource::{BindGroupLayout, Shader};
use bevy_asset::Handle;
use bevy_reflect::Uuid;
use bevy_utils::HashMap;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};
use wgpu::{
    BufferAddress, ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState,
    VertexAttribute, VertexFormat, VertexStepMode,
};

/// A [`RenderPipeline`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderPipelineId(Uuid);

/// A RenderPipeline represents a graphics pipeline and its stages (shaders), bindings and vertex buffers.
///
/// May be converted from and dereferences to a wgpu [`RenderPipeline`](wgpu::RenderPipeline).
/// Can be created via [`RenderDevice::create_render_pipeline`](crate::renderer::RenderDevice::create_render_pipeline).
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    id: RenderPipelineId,
    value: Arc<wgpu::RenderPipeline>,
}

impl RenderPipeline {
    #[inline]
    pub fn id(&self) -> RenderPipelineId {
        self.id
    }
}

impl From<wgpu::RenderPipeline> for RenderPipeline {
    fn from(value: wgpu::RenderPipeline) -> Self {
        RenderPipeline {
            id: RenderPipelineId(Uuid::new_v4()),
            value: Arc::new(value),
        }
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// A [`ComputePipeline`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ComputePipelineId(Uuid);

/// A ComputePipeline represents a compute pipeline and its single shader stage.
///
/// May be converted from and dereferences to a wgpu [`ComputePipeline`](wgpu::ComputePipeline).
/// Can be created via [`RenderDevice::create_compute_pipeline`](crate::renderer::RenderDevice::create_compute_pipeline).
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    value: Arc<wgpu::ComputePipeline>,
}

impl ComputePipeline {
    /// Returns the [`ComputePipelineId`].
    #[inline]
    pub fn id(&self) -> ComputePipelineId {
        self.id
    }
}

impl From<wgpu::ComputePipeline> for ComputePipeline {
    fn from(value: wgpu::ComputePipeline) -> Self {
        ComputePipeline {
            id: ComputePipelineId(Uuid::new_v4()),
            value: Arc::new(value),
        }
    }
}

impl Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Describes a render (graphics) pipeline.
#[derive(Clone, Debug)]
pub struct RenderPipelineDescriptor {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Option<Vec<BindGroupLayout>>,
    /// The compiled vertex stage, its entry point, and the input buffers layout.
    pub vertex: VertexState,
    /// The properties of the pipeline at the primitive assembly and rasterization level.
    pub primitive: PrimitiveState,
    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil: Option<DepthStencilState>,
    /// The multi-sampling properties of the pipeline.
    pub multisample: MultisampleState,
    /// The compiled fragment stage, its entry point, and the color targets.
    pub fragment: Option<FragmentState>,
}

#[derive(Clone, Debug)]
pub struct VertexState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<String>,
    /// The name of the entry point in the compiled shader. There must be a function that returns
    /// void with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The format of any vertex buffers used with this pipeline.
    pub buffers: Vec<VertexBufferLayout>,
}

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Default, Eq)]
pub struct VertexBufferLayout {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: BufferAddress,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    attributes: Vec<VertexAttributeLayout>,
    /// The list of attributes suitable for `wgpu`.
    wgpu_attributes: Vec<VertexAttribute>,
    /// Attribute names for debugging and mapping types.
    attribute_names: HashMap<String, usize>,
}

impl Hash for VertexBufferLayout {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.array_stride.hash(state);
        self.step_mode.hash(state);
        self.attributes.hash(state);
    }
}

impl PartialEq for VertexBufferLayout {
    fn eq(&self, other: &Self) -> bool {
        self.array_stride == other.array_stride
            && self.step_mode == other.step_mode
            && self.attributes == other.attributes
    }
}

impl VertexBufferLayout {
    /// Push a vertex attribute descriptor to the end of the list.
    ///
    /// The shader location is determined based on insertion order.
    pub fn push(&mut self, name: &str, format: VertexFormat) {
        let shader_location = self.attributes.last().map_or(0, |attr| attr.shader_location + 1);

        self.push_location(name, format, shader_location)
    }

    /// Push a vertex attribute descriptor to the end of the list with an exact shader location.
    pub fn push_location(&mut self, name: &str, format: VertexFormat, shader_location: u32) {
        let offset = self.attributes.last().map_or(0, |attr| attr.offset + attr.format.size());

        self.array_stride += format.size();
        self.attribute_names
            .entry(name.to_string())
            .or_insert_with(|| self.attributes.len());
        self.attributes.push(VertexAttributeLayout {
            name: name.to_string(),
            format,
            offset,
            shader_location,
        });
        self.wgpu_attributes.push(VertexAttribute {
            format,
            offset,
            shader_location,
        })
    }

    /// Get an attribute layout by name.
    pub fn attribute_layout(&self, name: &str) -> Option<&VertexAttributeLayout> {
        self.attribute_names.get(name).map(|i| &self.attributes[*i])
    }

    /// Get attributes suitable for `wgpu`.
    pub fn attributes(&self) -> &[VertexAttribute] {
        &self.wgpu_attributes
    }
}

/// Describes a vertex attribute's layout.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexAttributeLayout {
    /// The attribute's name.
    pub name: String,
    /// Format of the attribute.
    pub format: VertexFormat,
    /// Byte offset of this attribute within the array element.
    pub offset: u64,
    /// Attribute location as referenced by the shader.
    pub shader_location: u32,
}

impl From<&VertexAttributeLayout> for VertexAttribute {
    fn from(value: &VertexAttributeLayout) -> Self {
        Self {
            format: value.format,
            offset: value.offset,
            shader_location: value.shader_location,
        }
    }
}

/// Describes the fragment process in a render pipeline.
#[derive(Clone, Debug)]
pub struct FragmentState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<String>,
    /// The name of the entry point in the compiled shader. There must be a function that returns
    /// void with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The color state of the render targets.
    pub targets: Vec<ColorTargetState>,
}
