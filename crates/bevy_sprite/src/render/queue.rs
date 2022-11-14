use super::{DrawSprite, PreparedSprites, SpriteAssetEvents, SpritePipeline, SpritePipelineKey};
use bevy_asset::{AssetEvent, Handle, HandleId};
use bevy_core_pipeline::{core_2d::Transparent2d, tonemapping::Tonemapping};
use bevy_ecs::prelude::*;
use bevy_render::{
    prelude::{Image, Msaa},
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferUsages, BufferVec,
        PipelineCache, SpecializedRenderPipelines,
    },
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, ViewUniforms, VisibleEntities},
};
use bevy_utils::{FloatOrd, HashMap, Uuid};
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;
use std::cmp::Ordering;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct ColoredSpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Resource)]
pub struct SpriteMeta {
    pub(crate) vertices: BufferVec<SpriteVertex>,
    pub(crate) colored_vertices: BufferVec<ColoredSpriteVertex>,
    pub(crate) view_bind_group: Option<BindGroup>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            colored_vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct SpriteBatch {
    pub(crate) image_handle_id: HandleId,
    pub(crate) colored: bool,
}

#[derive(Resource, Default)]
pub struct ImageBindGroups {
    pub(crate) values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites(
    mut commands: Commands,
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    view_uniforms: Res<ViewUniforms>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpritePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut prepared_sprites: ResMut<PreparedSprites>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        &ExtractedView,
        Option<&Tonemapping>,
    )>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } | AssetEvent::Removed { handle } => {
                image_bind_groups.values.remove(handle)
            }
        };
    }

    let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples);

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let sprite_meta = &mut sprite_meta;

        // Clear the vertex buffers
        sprite_meta.vertices.clear();
        sprite_meta.colored_vertices.clear();

        sprite_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("sprite_view_bind_group"),
            layout: &sprite_pipeline.view_layout,
        }));

        let draw_sprite_function = draw_functions.read().get_id::<DrawSprite>().unwrap();

        // Vertex buffer indices
        let mut index = 0;
        let mut colored_index = 0;

        // FIXME: VisibleEntities is ignored

        let prepared_sprites = &mut prepared_sprites.sprites;
        // Sort sprites by z for correct transparency and then by handle to improve batching
        // NOTE: This can be done independent of views by reasonably assuming that all 2D views look along the negative-z axis in world space
        prepared_sprites.sort_unstable_by(|a, b| match a.sort_key.partial_cmp(&b.sort_key) {
            Some(Ordering::Equal) | None => a.image_handle_id.cmp(&b.image_handle_id),
            Some(other) => other,
        });
        let image_bind_groups = &mut *image_bind_groups;

        for (mut transparent_phase, visible_entities, view, tonemapping) in &mut views {
            let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;
            if let Some(Tonemapping::Enabled { deband_dither }) = tonemapping {
                if !view.hdr {
                    view_key |= SpritePipelineKey::TONEMAP_IN_SHADER;

                    if *deband_dither {
                        view_key |= SpritePipelineKey::DEBAND_DITHER;
                    }
                }
            }
            let pipeline = pipelines.specialize(
                &mut pipeline_cache,
                &sprite_pipeline,
                view_key | SpritePipelineKey::from_colored(false),
            );
            let colored_pipeline = pipelines.specialize(
                &mut pipeline_cache,
                &sprite_pipeline,
                view_key | SpritePipelineKey::from_colored(true),
            );

            view_entities.clear();
            view_entities.extend(visible_entities.entities.iter().map(|e| e.index() as usize));
            transparent_phase.items.reserve(prepared_sprites.len());

            // Impossible starting values that will be replaced on the first iteration
            let mut current_batch = SpriteBatch {
                image_handle_id: HandleId::Id(Uuid::nil(), u64::MAX),
                colored: false,
            };
            let mut current_batch_entity = Entity::from_raw(u32::MAX);
            // Add a phase item for each sprite, and detect when succesive items can be batched.
            // Spawn an entity with a `SpriteBatch` component for each possible batch.
            // Compatible items share the same entity.
            // Batches are merged later (in `batch_phase_system()`), so that they can be interrupted
            // by any other phase item (and they can interrupt other items from batching).
            for prepared_sprite in prepared_sprites.iter() {
                if !view_entities.contains(prepared_sprite.entity.index() as usize) {
                    continue;
                }
                let new_batch = SpriteBatch {
                    image_handle_id: prepared_sprite.image_handle_id,
                    colored: prepared_sprite.color != [1.0; 4],
                };
                if new_batch != current_batch {
                    // Set-up a new possible batch
                    if let Some(gpu_image) =
                        gpu_images.get(&Handle::weak(new_batch.image_handle_id))
                    {
                        current_batch = new_batch;
                        current_batch_entity = commands.spawn(current_batch).id();

                        image_bind_groups
                            .values
                            .entry(Handle::weak(current_batch.image_handle_id))
                            .or_insert_with(|| {
                                render_device.create_bind_group(&BindGroupDescriptor {
                                    entries: &[
                                        BindGroupEntry {
                                            binding: 0,
                                            resource: BindingResource::TextureView(
                                                &gpu_image.texture_view,
                                            ),
                                        },
                                        BindGroupEntry {
                                            binding: 1,
                                            resource: BindingResource::Sampler(&gpu_image.sampler),
                                        },
                                    ],
                                    label: Some("sprite_material_bind_group"),
                                    layout: &sprite_pipeline.material_layout,
                                })
                            });
                    } else {
                        // Skip this item if the texture is not ready
                        continue;
                    }
                }

                // These items will be sorted by depth with other phase items
                let sort_key = FloatOrd(prepared_sprite.sort_key);

                // Store the vertex data and add the item to the render phase
                if current_batch.colored {
                    for i in QUAD_INDICES {
                        sprite_meta.colored_vertices.push(ColoredSpriteVertex {
                            position: prepared_sprite.vertex_positions[i],
                            uv: prepared_sprite.vertex_uvs[i],
                            color: prepared_sprite.color,
                        });
                    }
                    let item_start = colored_index;
                    colored_index += QUAD_INDICES.len() as u32;
                    let item_end = colored_index;

                    transparent_phase.add(Transparent2d {
                        draw_function: draw_sprite_function,
                        pipeline: colored_pipeline,
                        entity: current_batch_entity,
                        sort_key,
                        batch_range: Some(item_start..item_end),
                    });
                } else {
                    for i in QUAD_INDICES {
                        sprite_meta.vertices.push(SpriteVertex {
                            position: prepared_sprite.vertex_positions[i],
                            uv: prepared_sprite.vertex_uvs[i],
                        });
                    }
                    let item_start = index;
                    index += QUAD_INDICES.len() as u32;
                    let item_end = index;

                    transparent_phase.add(Transparent2d {
                        draw_function: draw_sprite_function,
                        pipeline,
                        entity: current_batch_entity,
                        sort_key,
                        batch_range: Some(item_start..item_end),
                    });
                }
            }
        }
        sprite_meta
            .vertices
            .write_buffer(&render_device, &render_queue);
        sprite_meta
            .colored_vertices
            .write_buffer(&render_device, &render_queue);
    }
}
