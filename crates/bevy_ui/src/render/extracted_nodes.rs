use bevy_ecs::system::Resource;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_math::Vec4;
use bevy_render::color::Color;
use bevy_render::texture::Image;

use super::*;
use crate::instances::ExtractedInstance;
use crate::instances::NodeInstance;
use crate::instances::TextInstance;
use crate::instances::*;
use crate::prelude::*;

use super::TEXTURED_QUAD;
use super::UNTEXTURED_QUAD;

#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    pub uinodes: EntityHashMap<Entity, ExtractedItem>,
}

impl ExtractedUiNodes {
    pub fn push(&mut self, entity: Entity, item: ExtractedItem) {
        self.uinodes.insert(entity, item);
    }
}

pub struct ExtractedItem {
    pub stack_index: u32,
    pub image: AssetId<Image>,
    pub instance: ExtractedInstance,
}

impl ExtractedItem {
    fn new(
        stack_index: usize,
        image: AssetId<Image>,
        instance: impl Into<ExtractedInstance>,
    ) -> Self {
        Self {
            stack_index: stack_index as u32,
            image,
            instance: instance.into(),
        }
    }
}

impl ExtractedUiNodes {
    pub fn push_glyph(
        &mut self,
        entity: Entity,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        image: AssetId<Image>,
        color: Color,
        clip: Option<Rect>,
        uv_rect: Rect,
    ) {
        let color = color.as_linear_rgba_f32();
        let uv_min = uv_rect.min.into();
        let uv_size = uv_rect.size().into();
        let i = TextInstance {
            location: position.into(),
            size: size.into(),
            uv_min,
            uv_size,
            color,
        };
        self.push(entity, ExtractedItem::new(stack_index, image, (i, clip)));
    }

    pub fn push_node(
        &mut self,
        entity: Entity,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        image: Option<AssetId<Image>>,
        uv_rect: Rect,
        color: Color,
        border: [f32; 4],
        radius: [f32; 4],
        clip: Option<Rect>,
        flip_x: bool,
        flip_y: bool,
    ) {
        let color = color.as_linear_rgba_f32();
        let (uv_x, uv_w) = if flip_x {
            (uv_rect.max.x, -uv_rect.size().x)
        } else {
            (uv_rect.min.x, uv_rect.size().x)
        };

        let (uv_y, uv_h) = if flip_y {
            (uv_rect.max.y, -uv_rect.size().y)
        } else {
            (uv_rect.min.y, uv_rect.size().y)
        };

        let flags = if image.is_some() {
            TEXTURED_QUAD
        } else {
            UNTEXTURED_QUAD
        };

        let image = image.unwrap_or(AssetId::default());

        let i = NodeInstance {
            location: position.into(),
            size: size.into(),
            uv: [uv_x, uv_y, uv_w, uv_h],
            color,
            border,
            radius,
            flags,
        };
        self.push(entity, ExtractedItem::new(stack_index, image, (i, clip)));
    }

    pub fn push_border(
        &mut self,
        entity: Entity,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        color: Color,
        border: [f32; 4],
        radius: [f32; 4],
        clip: Option<Rect>,
    ) {
        if border.iter().all(|thickness| *thickness <= 0.) {
            return;
        }
        let color = color.as_linear_rgba_f32();
        let flags = UNTEXTURED_QUAD | BORDERED;
        let i = NodeInstance {
            location: position.into(),
            size: size.into(),
            uv: [0., 0., 1., 1.],
            border,
            color,
            radius,
            flags,
        };
        self.push(
            entity,
            ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
        );
    }

    pub fn push_dashed_border(
        &mut self,
        entity: Entity,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        color: Color,
        line_thickness: f32,
        dash_length: f32,
        break_length: f32,
        radius: [f32; 4],
        clip: Option<Rect>,
    ) {
        let color = color.as_linear_rgba_f32();
        let i = DashedBorderInstance {
            location: position.into(),
            size: size.into(),
            color,
            radius,
            width: line_thickness,
            dash_length,
            break_length,
        };
        self.push(
            entity,
            ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
        );
    }

    pub fn push_border_with_linear_gradient(
        &mut self,
        commands: &mut Commands,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        border: [f32; 4],
        radius: [f32; 4],
        start_point: Vec2,
        angle: f32,
        stops: &[(Color, f32)],
        clip: Option<Rect>,
    ) {
        if border.iter().all(|thickness| *thickness <= 0.) {
            return;
        }
        for i in 0..stops.len() - 1 {
            let start = &stops[i];
            let end = &stops[i + 1];

            let mut flags = UNTEXTURED_QUAD | BORDERED;
            if i == 0 {
                flags |= FILL_START;
            }

            if i + 2 == stops.len() {
                flags |= FILL_END;
            }

            let i = LinearGradientInstance {
                location: position.into(),
                size: size.into(),
                border,
                radius,
                flags,
                focal_point: start_point.into(),
                angle,
                start_color: start.0.as_linear_rgba_f32(),
                start_len: start.1,
                end_len: end.1,
                end_color: end.0.as_linear_rgba_f32(),
            };
            self.push(
                commands.spawn_empty().id(),
                ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
            );
        }
    }

    pub fn push_border_with_radial_gradient(
        &mut self,
        commands: &mut Commands,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        border: [f32; 4],
        radius: [f32; 4],
        ellipse: Ellipse,
        stops: &[(Color, f32)],
        clip: Option<Rect>,
    ) {
        if border.iter().all(|thickness| *thickness <= 0.) {
            return;
        }
        let start_point: Vec2 = (ellipse.center - position - 0.5 * size).into();
        let ratio = ellipse.extents.x / ellipse.extents.y;

        for i in 0..stops.len() - 1 {
            let start = &stops[i];
            let end = &stops[i + 1];

            let mut flags = UNTEXTURED_QUAD | BORDERED;
            if i == 0 {
                flags |= FILL_START;
            }

            if i + 2 == stops.len() {
                flags |= FILL_END;
            }

            let i = RadialGradientInstance {
                location: position.into(),
                size: size.into(),
                border,
                radius,
                flags,
                ratio,
                start_point: start_point.into(),
                start_color: start.0.as_linear_rgba_f32(),
                start_len: start.1,
                end_len: end.1,
                end_color: end.0.as_linear_rgba_f32(),
            };
            self.push(
                commands.spawn_empty().id(),
                ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
            );
        }
    }

    pub fn push_node_with_linear_gradient(
        &mut self,
        commands: &mut Commands,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        border: [f32; 4],
        radius: [f32; 4],
        start_point: Vec2,
        angle: f32,
        stops: &[(Color, f32)],
        clip: Option<Rect>,
    ) {
        let tflag = UNTEXTURED_QUAD;
        let image = AssetId::default();

        for i in 0..stops.len() - 1 {
            let start = &stops[i];
            let end = &stops[i + 1];
            let mut flags = tflag;
            if i == 0 {
                flags |= FILL_START;
            }

            if i + 2 == stops.len() {
                flags |= FILL_END;
            }

            let i = LinearGradientInstance {
                location: position.into(),
                size: size.into(),
                border,
                radius,
                flags,
                focal_point: start_point.into(),
                angle,
                start_color: start.0.as_linear_rgba_f32(),
                start_len: start.1,
                end_len: end.1,
                end_color: end.0.as_linear_rgba_f32(),
            };
            self.push(
                commands.spawn_empty().id(),
                ExtractedItem::new(stack_index, image.clone(), (i, clip)),
            );
        }
    }

    pub fn push_node_with_radial_gradient(
        &mut self,
        commands: &mut Commands,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        border: [f32; 4],
        radius: [f32; 4],
        ellipse: Ellipse,
        stops: &[(Color, f32)],
        clip: Option<Rect>,
    ) {
        let tflag = UNTEXTURED_QUAD;
        let start_point = (ellipse.center - position - 0.5 * size).into();

        let ratio = ellipse.extents.x / ellipse.extents.y;
        for i in 0..stops.len() - 1 {
            let start = &stops[i];
            let end = &stops[i + 1];
            let mut flags = tflag;
            if i == 0 {
                flags |= FILL_START;
            }

            if i + 2 == stops.len() {
                flags |= FILL_END;
            }

            let i = RadialGradientInstance {
                location: position.into(),
                size: size.into(),
                flags,
                border,
                radius,
                start_point,
                ratio,
                start_color: start.0.as_linear_rgba_f32(),
                start_len: start.1,
                end_len: end.1,
                end_color: end.0.as_linear_rgba_f32(),
            };
            self.push(
                commands.spawn_empty().id(),
                ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
            );
        }
    }

    pub fn push_shadow(
        &mut self,
        commands: &mut Commands,
        stack_index: usize,
        position: Vec2,
        size: Vec2,
        radius: [f32; 4],
        blur_radius: f32,
        color: Color,
        clip: Option<Rect>,
    ) {
        let color = color.as_linear_rgba_f32();

        let i = ShadowInstance {
            location: position.into(),
            size: size.into(),
            radius,
            color,
            blur_radius,
        };
        self.push(
            commands.spawn_empty().id(),
            ExtractedItem::new(stack_index, AssetId::default(), (i, clip)),
        );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, ShaderType, Default)]
struct UiClip {
    clip: Vec4,
}

#[derive(Resource)]
pub struct UiMeta {
    pub view_bind_group: Option<BindGroup>,
    pub index_buffer: BufferVec<u32>,
    pub instance_buffers: UiInstanceBuffers,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            index_buffer: BufferVec::<u32>::new(BufferUsages::INDEX),
            instance_buffers: Default::default(),
        }
    }
}

// impl UiMeta {
//     fn clear_instance_buffers(&mut self) {
//         self.instance_buffers.clear_all();
//     }

//     fn write_instance_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
//         self.instance_buffers.write_all(render_device, render_queue);
//     }

//     fn push(&mut self, item: &ExtractedInstance) {
//         item.push(&mut self.instance_buffers);
//     }
// }
