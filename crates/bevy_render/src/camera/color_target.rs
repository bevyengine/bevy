use alloc::sync::Arc;
use bevy_asset::{AssetId, Assets, Handle};
use bevy_camera::{
    color_target::{
        MainColorTarget, MainColorTargetInputConfig, MainColorTargetReadsFrom,
        NoAutoConfiguredMainColorTarget, WithMainColorTarget,
    },
    Camera, CameraMainColorTargetConfig, CameraMainColorTargetsSize, Hdr,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With, Without},
    relationship::RelationshipTarget,
    system::{Commands, Query, ResMut},
};
use bevy_image::{BevyDefault, Image, ToExtents};
use bevy_math::UVec2;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use wgpu::{TextureFormat, TextureUsages};

use crate::{extract_component::ExtractComponent, sync_world::RenderEntity, Extract};

pub(super) fn insert_camera_required_components_if_auto_configured(
    mut commands: Commands,
    query: Query<
        (Entity, Has<CameraMainColorTargetConfig>),
        (With<Camera>, Without<NoAutoConfiguredMainColorTarget>),
    >,
) {
    for (entity, has_config) in query.iter() {
        let mut entity_commands = commands.entity(entity);
        if !has_config {
            entity_commands.insert(CameraMainColorTargetConfig::default());
        }
    }
}

pub(super) fn configure_camera_color_target(
    mut commands: Commands,
    mut image_assets: ResMut<Assets<Image>>,
    query: Query<
        (
            Entity,
            &Camera,
            &CameraMainColorTargetConfig,
            Has<Hdr>,
            Option<&WithMainColorTarget>,
        ),
        Without<NoAutoConfiguredMainColorTarget>,
    >,
    mut main_color_targets: Query<&mut MainColorTarget>,
) {
    for (entity, camera, config, hdr, with_main_color_target) in query.iter() {
        let physical_size = camera.physical_target_size().unwrap_or(UVec2::ONE);
        let size = match config.size {
            CameraMainColorTargetsSize::Factor(vec2) => (physical_size.as_vec2() * vec2)
                .round()
                .as_uvec2()
                .max(UVec2::ONE),
            CameraMainColorTargetsSize::Fixed(uvec2) => uvec2,
        }
        .to_extents();
        let format = if let Some(format) = config.format {
            format
        } else if hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::bevy_default()
        };
        let mut image_desc = wgpu::TextureDescriptor {
            label: Some("main_texture_a"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: config.usage,
            view_formats: &[],
        };
        if let Some(with_main_color_target) = with_main_color_target {
            let Ok(mut main_color_target) = main_color_targets.get_mut(with_main_color_target.0)
            else {
                continue;
            };
            if config.sample_count > 1 {
                // If we already configured this camera but msaa is removed in `sync_camera_color_target_config`.
                // We need to re-add it.
                if main_color_target.multisampled.is_none() {
                    image_desc.label = Some("main_texture_multisampled");
                    image_desc.sample_count = config.sample_count;
                    image_desc.usage = TextureUsages::RENDER_ATTACHMENT;
                    let image_texture_multisampled = Image {
                        texture_descriptor: image_desc,
                        copy_on_resize: false,
                        data: None,
                        ..Default::default()
                    };
                    main_color_target.multisampled =
                        Some(image_assets.add(image_texture_multisampled));
                }
            } else {
                main_color_target.multisampled = None;
            }
        } else {
            let image_texture_a = Image {
                texture_descriptor: image_desc.clone(),
                copy_on_resize: false,
                data: None,
                ..Default::default()
            };
            image_desc.label = Some("main_texture_b");
            let image_texture_b = Image {
                texture_descriptor: image_desc.clone(),
                copy_on_resize: false,
                data: None,
                ..Default::default()
            };

            let msaa_texture = if config.sample_count > 1 {
                image_desc.label = Some("main_texture_multisampled");
                image_desc.sample_count = config.sample_count;
                image_desc.usage = TextureUsages::RENDER_ATTACHMENT;
                let image_texture_multisampled = Image {
                    texture_descriptor: image_desc,
                    copy_on_resize: false,
                    data: None,
                    ..Default::default()
                };
                Some(image_assets.add(image_texture_multisampled))
            } else {
                None
            };

            commands.entity(entity).insert((
                MainColorTarget::new(
                    image_assets.add(image_texture_a),
                    Some(image_assets.add(image_texture_b)),
                    msaa_texture,
                ),
                WithMainColorTarget(entity),
            ));
        }
    }
}

pub(super) fn sync_camera_color_target_config(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Camera,
            &CameraMainColorTargetConfig,
            Has<Hdr>,
            &WithMainColorTarget,
        ),
        Without<NoAutoConfiguredMainColorTarget>,
    >,
    mut query_main_color_targets: Query<&mut MainColorTarget>,
    mut image_assets: ResMut<Assets<Image>>,
) {
    for (entity, camera, config, hdr, with_color_target) in query.iter() {
        let Some(physical_size) = camera.physical_target_size() else {
            continue;
        };
        let size = match config.size {
            CameraMainColorTargetsSize::Factor(vec2) => (physical_size.as_vec2() * vec2)
                .round()
                .as_uvec2()
                .max(UVec2::ONE),
            CameraMainColorTargetsSize::Fixed(uvec2) => uvec2,
        }
        .to_extents();
        let Ok(mut main_textures) = query_main_color_targets.get_mut(with_color_target.0) else {
            continue;
        };
        let main_texture_a = main_textures.current_target();
        let main_texture_b = main_textures.other_target().unwrap();
        let format = if let Some(format) = config.format {
            format
        } else if hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::bevy_default()
        };
        let Some(main_texture_a) = image_assets.get_mut(main_texture_a) else {
            continue;
        };
        main_texture_a.resize(size);
        main_texture_a.texture_descriptor.usage = config.usage;
        main_texture_a.texture_descriptor.format = format;
        let Some(main_texture_b) = image_assets.get_mut(main_texture_b) else {
            continue;
        };
        main_texture_b.resize(size);
        main_texture_b.texture_descriptor.usage = config.usage;
        main_texture_b.texture_descriptor.format = format;

        if config.sample_count > 1 {
            let Some(msaa_texture) = main_textures.multisampled.as_ref() else {
                // Msaa is re-enabled after disabled. Reconfigure it.
                commands.entity(entity).remove::<WithMainColorTarget>();
                continue;
            };
            let Some(msaa_texture) = image_assets.get_mut(msaa_texture) else {
                continue;
            };
            msaa_texture.resize(size);
            msaa_texture.texture_descriptor.format = format;
            msaa_texture.texture_descriptor.sample_count = config.sample_count;
        } else {
            main_textures.multisampled = None;
        }
    }
}

#[derive(Component)]
pub struct ExtractedMainColorTarget {
    pub main_a: AssetId<Image>,
    pub main_b: Option<AssetId<Image>>,
    pub multisampled: Option<AssetId<Image>>,
    pub main_target_flag: Option<Arc<AtomicUsize>>,
}

impl ExtractedMainColorTarget {
    pub fn current_target(&self) -> AssetId<Image> {
        if let Some(main_target) = &self.main_target_flag
            && main_target.load(Ordering::SeqCst) == 1
        {
            self.main_b.unwrap()
        } else {
            self.main_a
        }
    }

    pub fn other_target(&self) -> Option<AssetId<Image>> {
        let Some(main_target) = &self.main_target_flag else {
            return None;
        };
        Some(if main_target.load(Ordering::SeqCst) == 1 {
            self.main_a
        } else {
            self.main_b.unwrap()
        })
    }
}

impl ExtractComponent for MainColorTarget {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = ExtractedMainColorTarget;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(Self::Out {
            main_a: item.main_a.id(),
            main_b: item.main_b.as_ref().map(Handle::id),
            multisampled: item.multisampled.as_ref().map(Handle::id),
            main_target_flag: item.main_target_flag.clone(),
        })
    }
}

#[derive(Component, Debug)]
pub struct ExtractedMainColorTargetReadsFrom(pub Vec<(AssetId<Image>, MainColorTargetInputConfig)>);

pub(super) fn extract_main_color_target_reads_from(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &MainColorTargetReadsFrom), With<Camera>>>,
    query_main_color_targets: Extract<
        Query<(&MainColorTarget, Option<&MainColorTargetInputConfig>)>,
    >,
) {
    for (entity, reads_from) in query.iter() {
        let mut images = reads_from
            .iter()
            .map(|entity| {
                let (t, input_config) = query_main_color_targets.get(entity).unwrap();
                (
                    t.current_target().id(),
                    input_config.cloned().unwrap_or(MainColorTargetInputConfig {
                        blend_state: None,
                        order: 0,
                    }),
                )
            })
            .collect::<Vec<_>>();
        images.sort_by_key(|a| a.1.order);

        commands
            .entity(entity)
            .insert(ExtractedMainColorTargetReadsFrom(images));
    }
}
