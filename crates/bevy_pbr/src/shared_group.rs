use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    schedule::ParallelSystemDescriptorCoercion,
    system::{Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_asset::{PrepareAssetLabel, RenderAssets},
    render_resource::{AsBindGroup, BindGroup, BindGroupLayout, OwnedBindingResource},
    renderer::RenderDevice,
    texture::{FallbackImage, Image},
    RenderApp, RenderStage,
};

pub struct SharedBindGroupPlugin<G: AsBindGroup + Resource>(PhantomData<G>);

impl<G: AsBindGroup + Resource> Default for SharedBindGroupPlugin<G> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<G: AsBindGroup + Resource> Plugin for SharedBindGroupPlugin<G> {
    fn build(&self, app: &mut App) {
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<SharedBindGroup<G>>()
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_shared_bind_group::<G>.after(PrepareAssetLabel::PreAssetPrepare),
            );
    }
}

#[derive(Resource)]
pub struct SharedBindGroup<G> {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: Option<BindGroup>,
    pub bindings: Vec<OwnedBindingResource>,
    marker: PhantomData<G>,
}

impl<G: AsBindGroup> FromWorld for SharedBindGroup<G> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            bind_group_layout: G::bind_group_layout(render_device),
            bind_group: None,
            bindings: Vec::new(),
            marker: PhantomData,
        }
    }
}

fn prepare_shared_bind_group<G: AsBindGroup + Resource>(
    mut shared_group_meta: ResMut<SharedBindGroup<G>>,
    shared_group_source: Option<Res<G>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
) {
    if shared_group_meta.bind_group.is_some() {
        return;
    }

    if let Some(shared_data) = shared_group_source {
        if let Ok(prepared) = shared_data.as_bind_group(
            &shared_group_meta.bind_group_layout,
            &render_device,
            &images,
            &fallback_image,
        ) {
            shared_group_meta.bind_group = Some(prepared.bind_group);
            shared_group_meta.bindings = prepared.bindings;
        }
    }
}
