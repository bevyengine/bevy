use bevy_core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Tick,
    resource::Resource,
    system::{Query, ResMut, SystemChangeTick},
};
use bevy_render::{
    sync_world::{MainEntity, MainEntityHashMap},
    view::{ExtractedView, Msaa},
};

use super::{pipeline::Mesh2dPipelineKey, tonemapping_pipeline_key};

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewKeyCache(MainEntityHashMap<Mesh2dPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewSpecializationTicks(MainEntityHashMap<Tick>);

pub(super) fn check_views_need_specialization(
    mut view_key_cache: ResMut<ViewKeyCache>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks>,
    views: Query<(
        &MainEntity,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
    ticks: SystemChangeTick,
) {
    for (view_entity, view, msaa, tonemapping, dither) in &views {
        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }

        if !view_key_cache
            .get_mut(view_entity)
            .is_some_and(|current_key| *current_key == view_key)
        {
            view_key_cache.insert(*view_entity, view_key);
            view_specialization_ticks.insert(*view_entity, ticks.this_run());
        }
    }
}
