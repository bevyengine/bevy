use crate::render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass};
use crate::render_resource::*;
use crate::renderer::RenderDevice;
use crate::{Plugin, RenderStage, TypeId};
use bevy_app::App;
use bevy_ecs::prelude::{
    Commands, Component, Entity, Mut, Query, Res, Resource, SystemLabel, World,
};
use bevy_ecs::query::ReadOnlyWorldQuery;
use bevy_ecs::schedule::IntoSystemDescriptor;
use bevy_ecs::system::lifetimeless::{Read, SQuery};
use bevy_ecs::system::{
    ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemParamItem, SystemState,
};
use bevy_log::{debug, warn};
use bevy_utils::HashMap;

use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::RwLock;

pub trait AutoBindGroup: Send + Sync + 'static {
    fn label() -> Option<&'static str> {
        None
    }
}

// a wgpu bind group layout entry, but with no explicit binding slot
pub struct AutoBindGroupLayoutEntry {
    /// Which shader stages can see this binding.
    pub visibility: ShaderStages,
    /// The type of the binding
    pub ty: BindingType,
    /// If this value is Some, indicates this entry is an array. Array size must be 1 or greater.
    ///
    /// If this value is Some and `ty` is `BindingType::Texture`, [`wgpu::Features::TEXTURE_BINDING_ARRAY`] must be supported.
    ///
    /// If this value is Some and `ty` is any other variant, bind group creation will fail.
    pub count: Option<NonZeroU32>,
}

impl From<AutoBindGroupLayoutEntry> for BindGroupLayoutEntry {
    fn from(entry: AutoBindGroupLayoutEntry) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: 0,
            visibility: entry.visibility,
            ty: entry.ty,
            count: entry.count,
        }
    }
}

// module name and variable name for a bound shader variable
#[derive(Debug)]
pub struct ShaderBindingName {
    pub module: Option<String>,
    pub name: String,
}

impl ShaderBindingName {
    pub fn new(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: Some(module.into()),
            name: name.into(),
        }
    }

    pub fn new_toplevel(name: impl Into<String>) -> Self {
        Self {
            module: None,
            name: name.into(),
        }
    }
}

#[derive(SystemLabel)]
pub enum AutoBindingStage {
    MakeBindingResource,
}

// manages construction of an automatic bind group
// G - name of the bindgroup
// F - filter to identify entities for which the bindgroup should be created
pub struct AutoBindGroupPlugin<G: AutoBindGroup, F: ReadOnlyWorldQuery + 'static> {
    _p: PhantomData<fn() -> (G, F)>,
}

impl<G: AutoBindGroup, F: ReadOnlyWorldQuery + 'static> Default for AutoBindGroupPlugin<G, F> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<G: AutoBindGroup, F: ReadOnlyWorldQuery + 'static> Plugin for AutoBindGroupPlugin<G, F> {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoBindingsIndex>()
            .init_resource::<AutoBindings<G>>()
            .add_system_to_stage(RenderStage::Prepare, prepare_binding_data_container::<G, F>)
            .add_system_to_stage(
                RenderStage::Queue,
                queue_auto_bindgroup::<G, F>.after(AutoBindingStage::MakeBindingResource),
            );
    }
}

trait DynAutoBinding: Send + Sync + 'static {
    fn bindgroup_layout_entry(&self, world: &mut World) -> BindGroupLayoutEntry;
}

// trait to define a binding from ECS data
pub trait AutoBinding: 'static {
    type LayoutParam: SystemParam + 'static;
    type BindingParam: SystemParam + 'static;

    fn bind_name() -> ShaderBindingName;
    fn bindgroup_layout_entry(
        param: SystemParamItem<Self::LayoutParam>,
    ) -> AutoBindGroupLayoutEntry;
    fn binding_source(
        entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource>;
}

#[derive(Resource, Default)]
struct AutoBindingImpl<B: AutoBinding> {
    index: u32,
    _p: PhantomData<fn() -> B>,
}

impl<B: AutoBinding> AutoBindingImpl<B> {
    fn new(index: u32) -> Self {
        Self {
            index,
            _p: Default::default(),
        }
    }
}

impl<B: AutoBinding> DynAutoBinding for AutoBindingImpl<B>
where
    <<B as AutoBinding>::LayoutParam as SystemParam>::Fetch: ReadOnlySystemParamFetch,
{
    fn bindgroup_layout_entry(&self, world: &mut World) -> BindGroupLayoutEntry {
        let mut state = SystemState::<B::LayoutParam>::new(world);
        let param = state.get(world);
        BindGroupLayoutEntry {
            binding: self.index,
            ..B::bindgroup_layout_entry(param).into()
        }
    }
}

#[derive(Resource, Default)]
pub struct AutoBindingsIndex {
    pub(crate) lookup: HashMap<String, u32>,
}

impl AutoBindingsIndex {
    fn get_or_insert(&mut self, binding_name: ShaderBindingName) -> u32 {
        let naga_oil_name = naga_oil::compose::Composer::decorated_name(
            binding_name.module.as_deref(),
            &binding_name.name,
        );

        match self.lookup.get(&naga_oil_name) {
            Some(index) => *index,
            None => {
                let index = self.lookup.len() as u32;
                debug!("binding slot allocated: {:?} -> {}", binding_name, index);
                self.lookup.insert(naga_oil_name, index);
                index
            }
        }
    }
}

#[derive(Resource)]
pub struct AutoBindings<G: AutoBindGroup> {
    bindings: Vec<Box<dyn DynAutoBinding>>,
    _p: PhantomData<fn() -> G>,
}

impl<G: AutoBindGroup> Default for AutoBindings<G> {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            _p: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct AutoBindingData<G: AutoBindGroup> {
    binding_sources: BTreeMap<u32, Option<OwnedBindingResource>>,
    _p: PhantomData<fn(&G)>,
}

fn prepare_binding_data_container<G: AutoBindGroup, F: ReadOnlyWorldQuery>(
    mut commands: Commands,
    entities: Query<Entity, F>,
) {
    for entity in &entities {
        commands.entity(entity).insert(AutoBindingData::<G> {
            binding_sources: Default::default(),
            _p: Default::default(),
        });
    }
}

#[derive(Component)]
pub struct AutoBindGroupData<G: AutoBindGroup> {
    bindgroup: BindGroup,
    offsets: Vec<u32>,
    _p: PhantomData<fn() -> G>,
}

// api for adding automatic bindgroups / bindings
pub trait AddAutoBinding {
    fn add_auto_binding<G: AutoBindGroup, B: AutoBinding>(&mut self) -> &mut Self
    where
        <<B as AutoBinding>::LayoutParam as SystemParam>::Fetch: ReadOnlySystemParamFetch,
        <<B as AutoBinding>::BindingParam as SystemParam>::Fetch: ReadOnlySystemParamFetch;
}

impl AddAutoBinding for App {
    fn add_auto_binding<G: AutoBindGroup, B: AutoBinding>(&mut self) -> &mut Self
    where
        <<B as AutoBinding>::LayoutParam as SystemParam>::Fetch: ReadOnlySystemParamFetch,
        <<B as AutoBinding>::BindingParam as SystemParam>::Fetch: ReadOnlySystemParamFetch,
    {
        let mut lookup = self.world.resource_mut::<AutoBindingsIndex>();
        let binding_name = B::bind_name();
        let index = lookup.get_or_insert(binding_name);
        let binding = AutoBindingImpl::<B>::new(index);

        let entries = self.world.resource_scope(
            |world: &mut World, mut auto_bindings: Mut<AutoBindings<G>>| {
                auto_bindings.bindings.push(Box::new(binding));

                auto_bindings
                    .bindings
                    .iter()
                    .map(|b| b.bindgroup_layout_entry(world))
                    .collect::<Vec<_>>()
            },
        );

        let render_device = self.world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: G::label(),
            entries: &entries,
        });

        let _removed = AUTO_BINDGROUPS
            .write()
            .expect("failed to write AUTO_BINDGROUPS hashmap")
            .insert(TypeId::of::<G>(), layout);

        // TODO check if the removed layout is in use. if so that would imply a pipeline was created
        // before this autobinding was added to the app, and so it will have an incorrect layout.
        // unfortunately we can't just check the bevy-side Arc because the wgpu resource
        // may still be in use even if the bevy resource is not.

        let queue_binding_with_index = move |
            query: Query<(Entity, &mut AutoBindingData<G>)>,
            param: <<<B as AutoBinding>::BindingParam as SystemParam>::Fetch as SystemParamFetch>::Item,
        | {
            queue_binding::<G, B>(query, param, index);
        };

        self.add_system_to_stage(
            RenderStage::Queue,
            queue_binding_with_index.label(AutoBindingStage::MakeBindingResource),
        );

        self
    }
}

fn queue_binding<G: AutoBindGroup, B: AutoBinding>(
    mut query: Query<(Entity, &mut AutoBindingData<G>)>,
    param: <<<B as AutoBinding>::BindingParam as SystemParam>::Fetch as SystemParamFetch>::Item,
    index: u32,
) {
    for (entity, mut container) in query.iter_mut() {
        let source = B::binding_source(entity, &param);
        container.binding_sources.insert(index, source);
    }
}

static AUTO_BINDGROUPS: Lazy<RwLock<HashMap<TypeId, BindGroupLayout>>> = Lazy::new(RwLock::default);

// context-free access to an auto-bindgroup layout
pub fn auto_layout<G: AutoBindGroup>() -> BindGroupLayout {
    let layout = AUTO_BINDGROUPS
        .read()
        .expect("failed to read AUTO_BINDGROUPS hashmap")
        .get(&TypeId::of::<G>())
        .expect("auto layout does not exist (has the AutoBindGroup plugin been added?)")
        .clone();

    layout
}

pub fn queue_auto_bindgroup<G: AutoBindGroup, F: ReadOnlyWorldQuery>(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AutoBindingData<G>), F>,
    render_device: Res<RenderDevice>,
) {
    for (entity, mut binding_data) in &mut query {
        let sources: Option<Vec<(u32, OwnedBindingResource)>> =
            std::mem::take(&mut binding_data.binding_sources)
                .into_iter()
                .map(|(index, source)| source.map(|source| (index, source)))
                .collect();

        // ensure we have all required bindings
        let Some(sources) = sources else { continue };

        // create bindgroup entries
        let entries = sources
            .iter()
            .map(|(ix, b)| BindGroupEntry {
                binding: *ix,
                resource: b.get_binding(),
            })
            .collect::<Vec<_>>();

        // create bindgroup
        let bindgroup = render_device.create_bind_group(&BindGroupDescriptor {
            label: G::label(),
            layout: &auto_layout::<G>(),
            entries: &entries,
        });

        // sources is sorted by binding index, so dynamic offsets will be correctly sorted
        let offsets = sources
            .iter()
            .flat_map(|(_, entry)| entry.dynamic_offset())
            .collect::<Vec<_>>();

        debug!("--queue {:?} -- {}", entity, std::any::type_name::<G>());
        debug!("layout: {:?}", auto_layout::<G>());
        debug!("entries: {:?}", entries);
        debug!("offsets: {:?}", offsets);
        debug!("bindgroup: {:?}", bindgroup);
        debug!("--end queue {:?}", entity);

        commands.entity(entity).insert(AutoBindGroupData::<G> {
            bindgroup,
            offsets,
            _p: Default::default(),
        });
    }
}

pub struct SetAutoBindGroup<G: AutoBindGroup + Component, const I: usize> {
    _p: PhantomData<fn() -> G>,
}
impl<G: AutoBindGroup + Component, const I: usize> EntityRenderCommand for SetAutoBindGroup<G, I> {
    type Param = SQuery<Read<AutoBindGroupData<G>>>;
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        view_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Ok(bindgroup_data) = view_query.get_inner(view) else {
            warn!("failed to get {} bindgroup data for view {:?} (the plugin filter may be incorrect)", std::any::type_name::<G>(), view);
            return RenderCommandResult::Failure;
        };

        pass.set_bind_group(I, &bindgroup_data.bindgroup, &bindgroup_data.offsets);

        RenderCommandResult::Success
    }
}
