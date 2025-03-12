use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    system::{lifetimeless::Read, Commands, Query, Res},
    world::FromWorld,
};
use bevy_render::{
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult},
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupLayout, BindGroupLayoutEntry,
        DynamicBindGroupEntries, DynamicBindGroupLayoutEntries, ShaderStages,
    },
    renderer::RenderDevice,
    view::{ExtractedView, ViewUniform, ViewUniforms},
};

#[derive(Component)]
pub(crate) struct ViewBindGroup(BindGroup);

/// very common layout: just the view uniform
#[derive(Resource)]
pub(crate) struct OnlyViewLayout(pub BindGroupLayout);

impl FromWorld for OnlyViewLayout {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout =
            render_device.create_bind_group_layout("mesh_view_layout", &view_layout_entries());

        Self(view_layout)
    }
}

pub(crate) struct SetViewBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetViewBindGroup<I> {
    type Param = ();

    type ViewQuery = Read<ViewBindGroup>;

    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        view: bevy_ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        _: Option<bevy_ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        _: bevy_ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy_render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &view.0, &[0]);
        RenderCommandResult::Success
    }
}

pub(crate) fn prepare_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    layout: Res<OnlyViewLayout>,
    views: Query<Entity, With<ExtractedView>>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    let entries = DynamicBindGroupEntries::new_with_indices(((0, view_binding.clone()),));
    for entity in &views {
        commands
            .entity(entity)
            .insert(ViewBindGroup(render_device.create_bind_group(
                "view_bind_group",
                &layout.0,
                &entries,
            )));
    }
}

pub(crate) fn view_layout_entries() -> Vec<BindGroupLayoutEntry> {
    DynamicBindGroupLayoutEntries::new_with_indices(
        ShaderStages::FRAGMENT,
        ((
            0,
            uniform_buffer::<ViewUniform>(true).visibility(ShaderStages::VERTEX_FRAGMENT),
        ),),
    )
    .to_vec()
}
