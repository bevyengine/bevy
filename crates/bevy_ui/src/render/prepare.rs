use crate::{ExtractedUiNodes, UiMeta};
use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_math::{Vec2, Vec3, Vec4Swizzles};
use bevy_render::texture::DEFAULT_IMAGE_HANDLE;
use bevy_render::{
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
};
use bytemuck::{Pod, Zeroable};
use std::ops::Range;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub mode: u32,
}

const QUAD_VERTEX_POSITIONS: [Vec3; 4] = [
    Vec3::new(-0.5, -0.5, 0.0),
    Vec3::new(0.5, -0.5, 0.0),
    Vec3::new(0.5, 0.5, 0.0),
    Vec3::new(-0.5, 0.5, 0.0),
];

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub image: Handle<Image>,
    pub z: f32,
}

const TEXTURED_QUAD: u32 = 0;
const UNTEXTURED_QUAD: u32 = 1;

pub fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMeta>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
) {
    ui_meta.vertices.clear();

    // sort by ui stack index, starting from the deepest node
    extracted_uinodes
        .uinodes
        .sort_by_key(|node| node.stack_index);

    let mut start = 0;
    let mut end = 0;
    let mut current_batch_image = DEFAULT_IMAGE_HANDLE.typed();
    let mut last_z = 0.0;

    #[inline]
    fn is_textured(image: &Handle<Image>) -> bool {
        image.id() != DEFAULT_IMAGE_HANDLE.id()
    }

    for extracted_uinode in extracted_uinodes.uinodes.drain(..) {
        let mode = if is_textured(&extracted_uinode.image) {
            if current_batch_image.id() != extracted_uinode.image.id() {
                if is_textured(&current_batch_image) && start != end {
                    commands.spawn(UiBatch {
                        range: start..end,
                        image: current_batch_image,
                        z: last_z,
                    });
                    start = end;
                }
                current_batch_image = extracted_uinode.image.clone_weak();
            }
            TEXTURED_QUAD
        } else {
            // Untextured `UiBatch`es are never spawned within the loop.
            // If all the `extracted_uinodes` are untextured a single untextured UiBatch will be spawned after the loop terminates.
            UNTEXTURED_QUAD
        };

        let mut uinode_rect = extracted_uinode.rect;

        let rect_size = uinode_rect.size().extend(1.0);

        // Specify the corners of the node
        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| (extracted_uinode.transform * (pos * rect_size).extend(1.)).xyz());

        // Calculate the effect of clipping
        // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
        let mut positions_diff = if let Some(clip) = extracted_uinode.clip {
            [
                Vec2::new(
                    f32::max(clip.min.x - positions[0].x, 0.),
                    f32::max(clip.min.y - positions[0].y, 0.),
                ),
                Vec2::new(
                    f32::min(clip.max.x - positions[1].x, 0.),
                    f32::max(clip.min.y - positions[1].y, 0.),
                ),
                Vec2::new(
                    f32::min(clip.max.x - positions[2].x, 0.),
                    f32::min(clip.max.y - positions[2].y, 0.),
                ),
                Vec2::new(
                    f32::max(clip.min.x - positions[3].x, 0.),
                    f32::min(clip.max.y - positions[3].y, 0.),
                ),
            ]
        } else {
            [Vec2::ZERO; 4]
        };

        let positions_clipped = [
            positions[0] + positions_diff[0].extend(0.),
            positions[1] + positions_diff[1].extend(0.),
            positions[2] + positions_diff[2].extend(0.),
            positions[3] + positions_diff[3].extend(0.),
        ];

        let transformed_rect_size = extracted_uinode.transform.transform_vector3(rect_size);

        // Don't try to cull nodes that have a rotation
        // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
        // In those two cases, the culling check can proceed normally as corners will be on
        // horizontal / vertical lines
        // For all other angles, bypass the culling check
        // This does not properly handles all rotations on all axis
        if extracted_uinode.transform.x_axis[1] == 0.0 {
            // Cull nodes that are completely clipped
            if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
            {
                continue;
            }
        }
        let uvs = if mode == UNTEXTURED_QUAD {
            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
        } else {
            let atlas_extent = extracted_uinode.atlas_size.unwrap_or(uinode_rect.max);
            if extracted_uinode.flip_x {
                std::mem::swap(&mut uinode_rect.max.x, &mut uinode_rect.min.x);
                positions_diff[0].x *= -1.;
                positions_diff[1].x *= -1.;
                positions_diff[2].x *= -1.;
                positions_diff[3].x *= -1.;
            }
            if extracted_uinode.flip_y {
                std::mem::swap(&mut uinode_rect.max.y, &mut uinode_rect.min.y);
                positions_diff[0].y *= -1.;
                positions_diff[1].y *= -1.;
                positions_diff[2].y *= -1.;
                positions_diff[3].y *= -1.;
            }
            [
                Vec2::new(
                    uinode_rect.min.x + positions_diff[0].x,
                    uinode_rect.min.y + positions_diff[0].y,
                ),
                Vec2::new(
                    uinode_rect.max.x + positions_diff[1].x,
                    uinode_rect.min.y + positions_diff[1].y,
                ),
                Vec2::new(
                    uinode_rect.max.x + positions_diff[2].x,
                    uinode_rect.max.y + positions_diff[2].y,
                ),
                Vec2::new(
                    uinode_rect.min.x + positions_diff[3].x,
                    uinode_rect.max.y + positions_diff[3].y,
                ),
            ]
            .map(|pos| pos / atlas_extent)
        };

        let color = extracted_uinode.color.as_linear_rgba_f32();
        for i in QUAD_INDICES {
            ui_meta.vertices.push(UiVertex {
                position: positions_clipped[i].into(),
                uv: uvs[i].into(),
                color,
                mode,
            });
        }

        last_z = extracted_uinode.transform.w_axis[2];
        end += QUAD_INDICES.len() as u32;
    }

    // if start != end, there is one last batch to process
    if start != end {
        commands.spawn(UiBatch {
            range: start..end,
            image: current_batch_image,
            z: last_z,
        });
    }

    ui_meta.vertices.write_buffer(&render_device, &render_queue);
}
