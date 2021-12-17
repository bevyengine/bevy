use crate::{
    render_phase::TrackedRenderPass,
    render_resource::{CachedPipelineId, RenderPipelineCache},
};
use bevy_app::App;
use bevy_ecs::{
    all_tuples,
    entity::Entity,
    system::{
        lifetimeless::SRes, ReadOnlySystemParamFetch, SystemParam, SystemParamItem, SystemState,
    },
    world::World,
};
use bevy_utils::HashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{any::TypeId, fmt::Debug, hash::Hash};

/// A draw function which is used to draw a specific [`PhaseItem`].
///
/// They are the the general form of drawing items, whereas [`RenderCommands`](RenderCommand)
/// are more modular.
pub trait Draw<P: PhaseItem>: Send + Sync + 'static {
    /// Draws the [`PhaseItem`] by issuing draw calls via the [`TrackedRenderPass`].
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    );
}

/// An item which will be drawn to the screen. A phase item should be queued up for rendering
/// during the [`RenderStage::Queue`](crate::RenderStage::Queue) stage.
/// Afterwards it will be sorted and rendered automatically  in the
/// [`RenderStage::PhaseSort`](crate::RenderStage::PhaseSort) stage and
/// [`RenderStage::Render`](crate::RenderStage::Render) stage, respectively.
pub trait PhaseItem: Send + Sync + 'static {
    /// The type used for ordering the items. The smallest values are drawn first.
    type SortKey: Ord;
    /// Determines the order in which the items are drawn during the corresponding [`RenderPhase`].
    fn sort_key(&self) -> Self::SortKey;
    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;
}

// TODO: make this generic?
/// /// A [`Draw`] function identifier.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DrawFunctionId(usize);

/// Stores all draw functions for the [`PhaseItem`] type.
/// For retrieval they are associated with their [`TypeId`].
pub struct DrawFunctionsInternal<P: PhaseItem> {
    pub draw_functions: Vec<Box<dyn Draw<P>>>,
    pub indices: HashMap<TypeId, DrawFunctionId>,
}

impl<P: PhaseItem> DrawFunctionsInternal<P> {
    /// Adds the [`Draw`] function and associates it to its own type.
    pub fn add<T: Draw<P>>(&mut self, draw_function: T) -> DrawFunctionId {
        self.add_with::<T, T>(draw_function)
    }

    /// Adds the [`Draw`] function and associates it to the type `T`
    pub fn add_with<T: 'static, D: Draw<P>>(&mut self, draw_function: D) -> DrawFunctionId {
        self.draw_functions.push(Box::new(draw_function));
        let id = DrawFunctionId(self.draw_functions.len() - 1);
        self.indices.insert(TypeId::of::<T>(), id);
        id
    }

    /// Retrieves the [`Draw`] function corresponding to the `id` mutably.
    pub fn get_mut(&mut self, id: DrawFunctionId) -> Option<&mut dyn Draw<P>> {
        self.draw_functions.get_mut(id.0).map(|f| &mut **f)
    }

    /// Retrieves the id of the [`Draw`] function corresponding to their associated type `T`.
    pub fn get_id<T: 'static>(&self) -> Option<DrawFunctionId> {
        self.indices.get(&TypeId::of::<T>()).copied()
    }
}

/// Stores all draw functions for the [`PhaseItem`] type hidden behind a reader-writer lock.
/// To access them the [`DrawFunctions::read`] and [`DrawFunctions::write`] methods are used.
pub struct DrawFunctions<P: PhaseItem> {
    internal: RwLock<DrawFunctionsInternal<P>>,
}

impl<P: PhaseItem> Default for DrawFunctions<P> {
    fn default() -> Self {
        Self {
            internal: RwLock::new(DrawFunctionsInternal {
                draw_functions: Vec::new(),
                indices: HashMap::default(),
            }),
        }
    }
}

impl<P: PhaseItem> DrawFunctions<P> {
    /// Accesses the draw functions in read mode.
    pub fn read(&self) -> RwLockReadGuard<'_, DrawFunctionsInternal<P>> {
        self.internal.read()
    }

    /// Accesses the draw functions in write mode.
    pub fn write(&self) -> RwLockWriteGuard<'_, DrawFunctionsInternal<P>> {
        self.internal.write()
    }
}

/// RenderCommand is a trait that runs an ECS query and produces one or more
/// [`TrackedRenderPass`] calls. Types implementing this trait can be composed (as tuples).
///
/// They can be registered as a [`Draw`] function via the
/// [`AddRenderCommand::add_render_command`] method.
///
/// # Example
/// The `DrawPbr` draw function is created from the following render command
/// tuple.  Const generics are used to set specific bind group locations:
///
/// ```ignore
/// pub type DrawPbr = (
///     SetItemPipeline,
///     SetMeshViewBindGroup<0>,
///     SetStandardMaterialBindGroup<1>,
///     SetTransformBindGroup<2>,
///     DrawMesh,
/// );
/// ```
pub trait RenderCommand<P: PhaseItem> {
    /// Specifies all ECS data required by [`RenderCommand::render`].
    /// All parameters have to be read only.
    type Param: SystemParam;

    /// Renders the [`PhaseItem`] by issuing draw calls via the [`TrackedRenderPass`].
    fn render<'w>(
        view: Entity,
        item: &P,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult;
}

pub enum RenderCommandResult {
    Success,
    Failure,
}

pub trait EntityRenderCommand {
    type Param: SystemParam;
    fn render<'w>(
        view: Entity,
        item: Entity,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult;
}

pub trait EntityPhaseItem: PhaseItem {
    fn entity(&self) -> Entity;
}

pub trait CachedPipelinePhaseItem: PhaseItem {
    fn cached_pipeline(&self) -> CachedPipelineId;
}

impl<P: EntityPhaseItem, E: EntityRenderCommand> RenderCommand<P> for E {
    type Param = E::Param;

    #[inline]
    fn render<'w>(
        view: Entity,
        item: &P,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        <E as EntityRenderCommand>::render(view, item.entity(), param, pass)
    }
}

pub struct SetItemPipeline;
impl<P: CachedPipelinePhaseItem> RenderCommand<P> for SetItemPipeline {
    type Param = SRes<RenderPipelineCache>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &P,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(pipeline) = pipeline_cache.into_inner().get(item.cached_pipeline()) {
            pass.set_render_pipeline(pipeline);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

macro_rules! render_command_tuple_impl {
    ($($name: ident),*) => {
        impl<P: PhaseItem, $($name: RenderCommand<P>),*> RenderCommand<P> for ($($name,)*) {
            type Param = ($($name::Param,)*);

            #[allow(non_snake_case)]
            fn render<'w>(
                _view: Entity,
                _item: &P,
                ($($name,)*): SystemParamItem<'w, '_, Self::Param>,
                _pass: &mut TrackedRenderPass<'w>,
            ) -> RenderCommandResult{
                $(if let RenderCommandResult::Failure = $name::render(_view, _item, $name, _pass) {
                    return RenderCommandResult::Failure;
                })*
                RenderCommandResult::Success
            }
        }
    };
}

all_tuples!(render_command_tuple_impl, 0, 15, C);

/// Wraps a [`RenderCommand`] into a state so that it can be used as a [`Draw`] function.
/// Therefore the [`RenderCommand::Param`] is queried from the ECS and passed to the command.
pub struct RenderCommandState<P: PhaseItem, C: RenderCommand<P>> {
    state: SystemState<C::Param>,
}

impl<P: PhaseItem, C: RenderCommand<P>> RenderCommandState<P, C> {
    pub fn new(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

impl<P: PhaseItem, C: RenderCommand<P> + Send + Sync + 'static> Draw<P> for RenderCommandState<P, C>
where
    <C::Param as SystemParam>::Fetch: ReadOnlySystemParamFetch,
{
    /// Prepares the ECS parameters for the wrapped [`RenderCommand`] and then renders it.
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    ) {
        let param = self.state.get(world);
        C::render(view, item, param, pass);
    }
}

/// Registers a [`RenderCommand`] as a [`Draw`] function.
/// They are stored inside the [`DrawFunctions`] resource of the app.
pub trait AddRenderCommand {
    /// Adds the [`RenderCommand`] for the specified [`RenderPhase`](super::RenderPhase) to the app.
    fn add_render_command<P: PhaseItem, C: RenderCommand<P> + Send + Sync + 'static>(
        &mut self,
    ) -> &mut Self
    where
        <C::Param as SystemParam>::Fetch: ReadOnlySystemParamFetch;
}

impl AddRenderCommand for App {
    fn add_render_command<P: PhaseItem, C: RenderCommand<P> + Send + Sync + 'static>(
        &mut self,
    ) -> &mut Self
    where
        <C::Param as SystemParam>::Fetch: ReadOnlySystemParamFetch,
    {
        let draw_function = RenderCommandState::<P, C>::new(&mut self.world);
        let draw_functions = self.world.get_resource::<DrawFunctions<P>>().unwrap();
        draw_functions.write().add_with::<C, _>(draw_function);
        self
    }
}
