use bevy_math::Rect;
use bevy_render::{
    render_resource::{BufferUsages, BufferVec},
    renderer::{RenderDevice, RenderQueue},
};
use bytemuck::{Pod, Zeroable};

use crate::rect_to_f32_4;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct NodeInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub flags: u32,
    pub border: [f32; 4],
    pub radius: [f32; 4],
    pub color: [f32; 4],
    pub uv: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct TextInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_size: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct LinearGradientInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub flags: u32,
    pub border: [f32; 4],
    pub radius: [f32; 4],
    pub focal_point: [f32; 2],
    pub angle: f32,
    pub start_color: [f32; 4],
    pub start_len: f32,
    pub end_len: f32,
    pub end_color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct RadialGradientInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub flags: u32,
    pub border: [f32; 4],
    pub radius: [f32; 4],
    pub start_point: [f32; 2],
    pub ratio: f32,
    pub start_color: [f32; 4],
    pub start_len: f32,
    pub end_len: f32,
    pub end_color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct DashedBorderInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub width: f32,
    pub color: [f32; 4],
    pub radius: [f32; 4],
    pub dash_length: f32,
    pub break_length: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct ShadowInstance {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub radius: [f32; 4],
    pub color: [f32; 4],
    pub blur_radius: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Debug)]
pub struct ClippedInstance<I>
where
    I: Copy + Clone + Pod + Zeroable + std::fmt::Debug,
{
    pub instance: I,
    clip: [f32; 4],
}

impl<I> From<(I, [f32; 4])> for ClippedInstance<I>
where
    I: Clone + Copy + Pod + Zeroable + std::fmt::Debug,
{
    fn from((instance, clip): (I, [f32; 4])) -> Self {
        ClippedInstance { instance, clip }
    }
}

unsafe impl<I> Pod for ClippedInstance<I> where I: Clone + Copy + Pod + Zeroable + std::fmt::Debug {}

pub struct UiInstanceBuffer<I>
where
    I: Copy + Clone + Pod + Zeroable + std::fmt::Debug,
{
    pub clipped: BufferVec<ClippedInstance<I>>,
    pub unclipped: BufferVec<I>,
}

impl<I> UiInstanceBuffer<I>
where
    I: Copy + Clone + Pod + Zeroable + std::fmt::Debug,
{
    #[inline]
    pub fn clear(&mut self) {
        self.clipped.clear();
        self.unclipped.clear();
    }

    #[inline]
    fn write(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        self.clipped.write_buffer(render_device, render_queue);
        self.unclipped.write_buffer(render_device, render_queue);
    }
}

impl<I> Default for UiInstanceBuffer<I>
where
    I: Copy + Clone + Pod + Zeroable + std::fmt::Debug,
{
    fn default() -> Self {
        Self {
            clipped: BufferVec::<ClippedInstance<I>>::new(BufferUsages::VERTEX),
            unclipped: BufferVec::<I>::new(BufferUsages::VERTEX),
        }
    }
}

#[derive(Default)]
pub struct UiInstanceBuffers {
    pub node: UiInstanceBuffer<NodeInstance>,
    pub text: UiInstanceBuffer<TextInstance>,
    pub linear_gradient: UiInstanceBuffer<LinearGradientInstance>,
    pub radial_gradient: UiInstanceBuffer<RadialGradientInstance>,
    pub dashed_border: UiInstanceBuffer<DashedBorderInstance>,
    pub shadow: UiInstanceBuffer<ShadowInstance>,
}

impl UiInstanceBuffers {
    /// Clear all the instance buffers
    pub fn clear_all(&mut self) {
        self.node.clear();
        self.text.clear();
        self.linear_gradient.clear();
        self.radial_gradient.clear();
        self.dashed_border.clear();
        self.shadow.clear()
    }

    /// Queue writes for all instance buffers.
    /// Queues writing of data from system RAM to VRAM using the RenderDevice and the provided RenderQueue.
    /// Before queuing the write, a reserve operation is executed.
    pub fn write_all(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        self.node.write(&render_device, &render_queue);
        self.text.write(&render_device, &render_queue);
        self.linear_gradient.write(&render_device, &render_queue);
        self.radial_gradient.write(&render_device, &render_queue);
        self.dashed_border.write(&render_device, &render_queue);
        self.shadow.write(&render_device, &render_queue);
    }
}

pub trait UiInstance {
    fn push(self, buffers: &mut UiInstanceBuffers);
}

impl UiInstance for NodeInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.node.unclipped.push(self);
    }
}

impl UiInstance for TextInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.text.unclipped.push(self);
    }
}

impl UiInstance for LinearGradientInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.linear_gradient.unclipped.push(self);
    }
}

impl UiInstance for RadialGradientInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.radial_gradient.unclipped.push(self);
    }
}

impl UiInstance for ClippedInstance<NodeInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.node.clipped.push(self);
    }
}

impl UiInstance for ClippedInstance<TextInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.text.clipped.push(self);
    }
}

impl UiInstance for ClippedInstance<LinearGradientInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.linear_gradient.clipped.push(self);
    }
}

impl UiInstance for ClippedInstance<RadialGradientInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.radial_gradient.clipped.push(self);
    }
}

impl UiInstance for DashedBorderInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.dashed_border.unclipped.push(self);
    }
}

impl UiInstance for ClippedInstance<DashedBorderInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.dashed_border.clipped.push(self);
    }
}

impl UiInstance for ShadowInstance {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.shadow.unclipped.push(self);
    }
}

impl UiInstance for ClippedInstance<ShadowInstance> {
    #[inline]
    fn push(self, buffers: &mut UiInstanceBuffers) {
        buffers.shadow.clipped.push(self);
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BatchType {
    Node = 0,
    Text = 1,
    CNode = 2,
    CText = 3,
    LinearGradient = 4,
    CLinearGradient = 5,
    RadialGradient = 6,
    CRadialGradient = 7,
    DashedBorder = 8,
    CDashedBorder = 9,
    Shadow = 10,
    CShadow = 11,
}

#[derive(Default)]
pub struct InstanceCounters([u32; 12]);

impl InstanceCounters {
    #[inline]
    pub fn increment(&mut self, batch_type: BatchType) -> u32 {
        let value = &mut self.0[batch_type as usize];
        *value += 1;
        *value
    }
}
pub enum ExtractedInstance {
    Node(NodeInstance),
    Text(TextInstance),
    LinearGradient(LinearGradientInstance),
    RadialGradient(RadialGradientInstance),
    DashedBorder(DashedBorderInstance),
    CNode(ClippedInstance<NodeInstance>),
    CText(ClippedInstance<TextInstance>),
    CLinearGradient(ClippedInstance<LinearGradientInstance>),
    CRadialGradient(ClippedInstance<RadialGradientInstance>),
    CDashedBorder(ClippedInstance<DashedBorderInstance>),
    Shadow(ShadowInstance),
    CShadow(ClippedInstance<ShadowInstance>),
}

impl ExtractedInstance {
    pub fn get_type(&self) -> BatchType {
        match self {
            ExtractedInstance::Node(_) => BatchType::Node,
            ExtractedInstance::Text(_) => BatchType::Text,
            ExtractedInstance::CNode(_) => BatchType::CNode,
            ExtractedInstance::CText(_) => BatchType::CText,
            ExtractedInstance::LinearGradient(_) => BatchType::LinearGradient,
            ExtractedInstance::CLinearGradient(_) => BatchType::CLinearGradient,
            ExtractedInstance::RadialGradient(_) => BatchType::RadialGradient,
            ExtractedInstance::CRadialGradient(_) => BatchType::CRadialGradient,
            ExtractedInstance::DashedBorder(_) => BatchType::DashedBorder,
            ExtractedInstance::CDashedBorder(_) => BatchType::CDashedBorder,
            ExtractedInstance::Shadow(_) => BatchType::Shadow,
            ExtractedInstance::CShadow(_) => BatchType::CShadow,
        }
    }

    pub fn push(&self, instance_buffers: &mut UiInstanceBuffers) {
        match self {
            ExtractedInstance::Node(i) => i.push(instance_buffers),
            ExtractedInstance::Text(i) => i.push(instance_buffers),
            ExtractedInstance::LinearGradient(i) => i.push(instance_buffers),
            ExtractedInstance::RadialGradient(i) => i.push(instance_buffers),
            ExtractedInstance::CNode(i) => i.push(instance_buffers),
            ExtractedInstance::CText(i) => i.push(instance_buffers),
            ExtractedInstance::CLinearGradient(i) => i.push(instance_buffers),
            ExtractedInstance::CRadialGradient(i) => i.push(instance_buffers),
            ExtractedInstance::DashedBorder(i) => i.push(instance_buffers),
            ExtractedInstance::CDashedBorder(i) => i.push(instance_buffers),
            ExtractedInstance::Shadow(i) => i.push(instance_buffers),
            ExtractedInstance::CShadow(i) => i.push(instance_buffers),
        }
    }
}

#[inline]
fn get_clip(clip: Option<Rect>) -> Option<[f32; 4]> {
    clip.map(|clip| rect_to_f32_4(clip))
}

impl From<(NodeInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (NodeInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CNode((instance, clip).into())
        } else {
            Self::Node(instance)
        }
    }
}

impl From<(TextInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (TextInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CText((instance, clip).into())
        } else {
            Self::Text(instance)
        }
    }
}

impl From<(LinearGradientInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (LinearGradientInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CLinearGradient((instance, clip).into())
        } else {
            Self::LinearGradient(instance)
        }
    }
}

impl From<(RadialGradientInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (RadialGradientInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CRadialGradient((instance, clip).into())
        } else {
            Self::RadialGradient(instance)
        }
    }
}

impl From<(DashedBorderInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (DashedBorderInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CDashedBorder((instance, clip).into())
        } else {
            Self::DashedBorder(instance)
        }
    }
}

impl From<(ShadowInstance, Option<Rect>)> for ExtractedInstance {
    fn from((instance, clip): (ShadowInstance, Option<Rect>)) -> Self {
        if let Some(clip) = get_clip(clip) {
            Self::CShadow((instance, clip).into())
        } else {
            Self::Shadow(instance)
        }
    }
}
