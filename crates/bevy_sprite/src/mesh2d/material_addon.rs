use std::marker::PhantomData;
use bevy_app::App;
use bevy_asset::AssetServer;
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::prelude::{IntoSystemConfig, Resource};
use bevy_ecs::query::ROQueryItem;
use bevy_ecs::system::lifetimeless::SRes;
use bevy_ecs::system::SystemParamItem;
use bevy_render::mesh::MeshVertexBufferLayout;
use bevy_render::render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass};
use bevy_render::render_resource::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer, BufferDescriptor, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy_render::{RenderApp, RenderSet};
use bevy_render::renderer::RenderDevice;

pub trait MaterialAddon: Clone + Send + Sync + 'static {
    fn create_bind_group_layout_entry(asset_server: &AssetServer, binding: u32) -> BindGroupLayoutEntry;
    fn build_addon<G: Send + Sync + 'static, const BINDING_INDEX: usize>(app: &mut App);
    fn create_buffer_descriptor<'a>() -> BufferDescriptor<'a>;
}

pub trait MaterialAddonGroup<const GROUP_INDEX: usize>: Clone + Sized + Send + Sync + 'static
{
    type RenderCommandGroup: RenderCommand<Transparent2d> + Send + Sync + 'static;
    fn create_bind_group_layout(asset_server: &AssetServer, render_device: &RenderDevice) -> BindGroupLayout;
    fn build_addon_group<C: AddonCollectionMeta + 'static, P: DataHolder<C> + Resource, const GROUP_START_INDEX: usize>(app: &mut App);
}

pub trait DataHolder<T> { fn get(&self) -> &T; }
pub trait AddonCollectionMeta { fn layout_slice(&self) -> &[BindGroupLayout]; }
impl<M, const N: usize> AddonCollectionMeta for CollectionMeta<M, N> {
    fn layout_slice(&self) -> &[BindGroupLayout] {
        &self.layouts
    }
}

impl AddonCollectionMeta for () {
    fn layout_slice(&self) -> &[BindGroupLayout] {
        &[]
    }
}

pub trait MaterialAddonCollection<const GROUP_A: usize, const GROUP_B: usize, const GROUP_C: usize>: Send + Sync + 'static + Sized
{
    type AddonRenderCommandGroups: RenderCommand<Transparent2d> + Send + Sync + 'static;
    type Meta: AddonCollectionMeta + Send + Sync + 'static + Sized + Clone;

    fn create_bind_group_layouts(asset_server: &AssetServer, render_device: &RenderDevice) -> Self::Meta;
    fn build_addon_collection<P: DataHolder<Self::Meta> + Resource>(app: &mut App);

    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        meta: &Self::Meta,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut collected = meta.layout_slice().iter().map(|x| x.clone()).collect();
        descriptor.layout.append(&mut collected);
        Ok(())
    }
}

impl<A: MaterialAddon, const GROUP_INDEX: usize> MaterialAddonGroup<GROUP_INDEX> for A {
    type RenderCommandGroup = AddonBindGroup<GROUP_INDEX, Self>;

    fn create_bind_group_layout(asset_server: &AssetServer, render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                A::create_bind_group_layout_entry(asset_server, 0),
            ],
            label: Some(format!("addon_layout({})", std::any::type_name::<A>()).as_str()),
        })
    }

    fn build_addon_group<C: AddonCollectionMeta + 'static, P: DataHolder<C> + Resource, const GROUP_START_INDEX: usize>(app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>();
        let buffers = vec![
            render_device.create_buffer(&A::create_buffer_descriptor()),
        ];

        app.sub_app_mut(RenderApp)
            .insert_resource(AddonMeta::<Self> { data: Default::default(), buffers, bind_group: None, })
            .add_system(queue_bind_group::<Self, C, P, GROUP_START_INDEX, GROUP_INDEX>.in_set(RenderSet::Queue));
        A::build_addon::<Self, 0>(app);
    }
}

impl<A: MaterialAddon, B: MaterialAddon, const GROUP_INDEX: usize> MaterialAddonGroup<GROUP_INDEX> for (A, B) {
    type RenderCommandGroup = AddonBindGroup<GROUP_INDEX, Self>;
    fn create_bind_group_layout(asset_server: &AssetServer, render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                A::create_bind_group_layout_entry(asset_server, 0),
                B::create_bind_group_layout_entry(asset_server, 1)
            ],
            label: Some(format!("addon_layout({}, {})", std::any::type_name::<A>(), std::any::type_name::<B>()).as_str()),
        })
    }

    fn build_addon_group<C: AddonCollectionMeta + 'static, P: DataHolder<C> + Resource, const GROUP_START_INDEX: usize>(app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>();
        let buffers = vec![
            render_device.create_buffer(&A::create_buffer_descriptor()),
            render_device.create_buffer(&B::create_buffer_descriptor())
        ];

        app.sub_app_mut(RenderApp)
            .insert_resource(AddonMeta::<Self> { data: Default::default(), buffers, bind_group: None })
            .add_system(queue_bind_group::<Self, C, P, GROUP_START_INDEX, GROUP_INDEX>.in_set(RenderSet::Queue));

        A::build_addon::<Self, 0>(app);
        B::build_addon::<Self, 1>(app);
    }
}

impl<const GROUP_A: usize, const GROUP_B: usize, const GROUP_C: usize> MaterialAddonCollection<GROUP_A, GROUP_B, GROUP_C> for () {
    type AddonRenderCommandGroups = ();
    type Meta = ();
    fn create_bind_group_layouts(asset_server: &AssetServer, render_device: &RenderDevice) -> Self::Meta { () }
    fn build_addon_collection<P: DataHolder<Self::Meta> + Resource>(app: &mut App) {}
}

impl<A: MaterialAddonGroup<GROUP_A>, const GROUP_A: usize, const GROUP_B: usize, const GROUP_C: usize> MaterialAddonCollection<GROUP_A, GROUP_B, GROUP_C> for (A, ()) {
    type AddonRenderCommandGroups = A::RenderCommandGroup;
    type Meta = CollectionMeta<Self, 1>;

    fn create_bind_group_layouts(asset_server: &AssetServer, render_device: &RenderDevice) -> Self::Meta {
        Self::Meta {
            data: Default::default(),
            layouts: [A::create_bind_group_layout(asset_server, render_device)],
        }
    }

    fn build_addon_collection<P: DataHolder<Self::Meta> + Resource>(app: &mut App) {
        debug_assert!(GROUP_A != usize::MAX, "First group index must be set for {}", std::any::type_name::<Self>());
        A::build_addon_group::<Self::Meta, P, GROUP_A>(app);
    }
}

impl<
    A: MaterialAddonGroup<GROUP_A>, B: MaterialAddonGroup<GROUP_B>, const GROUP_A: usize, const GROUP_B: usize, const GROUP_C: usize,
> MaterialAddonCollection<GROUP_A, GROUP_B, GROUP_C> for (A, B) {
    type AddonRenderCommandGroups = (A::RenderCommandGroup, B::RenderCommandGroup);
    type Meta = CollectionMeta<Self, 2>;
    fn create_bind_group_layouts(asset_server: &AssetServer, render_device: &RenderDevice) -> Self::Meta {
        Self::Meta {
            data: Default::default(),
            layouts: [
                A::create_bind_group_layout(asset_server, render_device),
                B::create_bind_group_layout(asset_server, render_device),
            ],
        }
    }

    fn build_addon_collection<P: DataHolder<Self::Meta> + Resource>(app: &mut App) {
        debug_assert!(GROUP_A != usize::MAX, "First group index must be set for {}", std::any::type_name::<Self>());
        debug_assert!(GROUP_B != usize::MAX, "Second group index must be set for {}", std::any::type_name::<Self>());
        debug_assert!(GROUP_A != GROUP_B, "Index groups must be different for {}", std::any::type_name::<Self>());
        A::build_addon_group::<Self::Meta, P, GROUP_A>(app);
        B::build_addon_group::<Self::Meta, P, GROUP_A>(app);
    }
}

pub struct AddonBindGroup<const GROUP: usize, G> (PhantomData<G>);
impl<P: PhaseItem, G: Send + Sync + 'static, const GROUP: usize> RenderCommand<P> for AddonBindGroup<GROUP, G> {
    type Param = SRes<AddonMeta<G>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewWorldQuery>,
        entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>
    ) -> RenderCommandResult {
        let bind_group = param.into_inner().bind_group.as_ref().unwrap();
        pass.set_bind_group(GROUP, bind_group, &[]);
        RenderCommandResult::Success
    }
}

#[derive(Resource)]
pub struct AddonMeta<G> {
    data: PhantomData<G>,
    pub(crate) buffers: Vec<Buffer>,
    pub(crate) bind_group: Option<BindGroup>,
}

#[derive(Debug, Clone)]
pub struct CollectionMeta<C: ?Sized, const N: usize> {
    data: PhantomData<C>,
    layouts: [BindGroupLayout; N],
}

fn queue_bind_group<
    G: Send + Sync + 'static,
    C: AddonCollectionMeta,
    P: DataHolder<C> + Resource,
    const GROUP_START_INDEX: usize,
    const GROUP_INDEX: usize
>(
    render_device: Res<RenderDevice>,
    pipeline: Res<P>,
    mut addon_meta: ResMut<AddonMeta<G>>,
) {
    let entries = addon_meta.buffers.iter().enumerate().map(|(i, b)| BindGroupEntry {
        binding: i as u32,
        resource: b.as_entire_binding(),
    }).collect::<Vec<BindGroupEntry>>();
    let layout = &pipeline.get().layout_slice()[GROUP_INDEX - GROUP_START_INDEX];
    addon_meta.bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout,
        entries: entries.as_slice(),
    }));
}