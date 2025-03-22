use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Tick, resource::Resource};
use bevy_render::sync_world::MainEntityHashMap;

use super::pipeline::Mesh2dPipelineKey;

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
/// Cache of [`MainEntity`](bevy_render::sync_world::MainEntity) and [`Mesh2dPipelineKey`]
pub struct ViewKeyCache(MainEntityHashMap<Mesh2dPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
/// Cache of [`MainEntity`](bevy_render::sync_world::MainEntity) and [`Tick`]
pub struct ViewSpecializationTicks(MainEntityHashMap<Tick>);
