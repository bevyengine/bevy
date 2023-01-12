use crate::render_phase::{PhaseItem, RenderPhase, TrackedRenderPass};
use bevy_app::App;
use bevy_ecs::{
    all_tuples,
    entity::Entity,
    query::{QueryState, ROQueryItem, ReadOnlyWorldQuery},
    system::{ReadOnlySystemParam, Resource, SystemParamItem, SystemState},
    world::World,
};
use bevy_utils::HashMap;
use parking_lot::RwLock;
use std::{any::TypeId, fmt::Debug, hash::Hash};

/// The result of a [`RenderCommand`].
pub enum RenderCommandResult {
    Success,
    Failure,
}

// TODO: make this generic?
/// An identifier of a [`RenderCommand`] stored in the [`RenderCommands`] collection.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RenderCommandId(u32);

/// [`RenderCommand`]s are modular pieces of render logic that are used to render [`PhaseItem`]s.
///
/// These phase items are rendered during a [`RenderPhase`] for a specific view,
/// by recording commands (e.g. setting pipelines, binding bind groups,
/// setting vertex/index buffers, and issuing draw calls) via the [`TrackedRenderPass`].
///
/// The read only ECS data, required by the [`render`](Self::render) method, is fetch automatically,
/// from the render world, using the [`Param`](Self::Param),
/// [`ViewWorldQuery`](Self::ViewWorldQuery), and [`ItemWorldQuery`](Self::ItemWorldQuery).
/// These three parameters are used to access render world resources,
/// components of the view entity, and components of the item entity respectively.
///
/// Before they can be used, render commands have to be registered on the render app via the
/// [`AddRenderCommand::add_render_command`] method.
///
/// Multiple render commands can be combined together by wrapping them in a tuple.
///
/// # Example
/// The `DrawPbr` render command is composed of the following render command tuple.
/// Const generics are used to set specific bind group locations:
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
pub trait RenderCommand<P: PhaseItem>: Send + Sync + 'static {
    /// Specifies the general ECS data (e.g. resources) required by [`Self::render`].
    ///
    /// When fetching resources, note that, due to lifetime limitations of the `Deref` trait,
    /// [`SRes::into_inner`] must be called on each [`SRes`] reference in the
    /// [`RenderCommand::render`] method, instead of being automatically dereferenced as is the
    /// case in normal `systems`.
    ///
    /// All parameters have to be read only.
    ///
    /// [`SRes`]: bevy_ecs::system::lifetimeless::SRes
    /// [`SRes::into_inner`]: bevy_ecs::system::lifetimeless::SRes::into_inner
    type Param: ReadOnlySystemParam;
    /// Specifies the ECS data of the view entity required by [`Self::render`].
    ///
    /// The view entity refers to the camera, or shadow-casting light, etc. from which the phase
    /// item will be rendered from.
    /// All components have to be accessed read only.
    type ViewWorldQuery: ReadOnlyWorldQuery;
    /// Specifies the ECS data of the item entity required by [`RenderCommand::render`].
    ///
    /// The item is the entity that will be rendered for the corresponding view.
    /// All components have to be accessed read only.
    type ItemWorldQuery: ReadOnlyWorldQuery;

    /// Renders a [`PhaseItem`] by recording commands (e.g. setting pipelines, binding bind groups,
    /// setting vertex/index buffers, and issuing draw calls) via the [`TrackedRenderPass`].
    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewWorldQuery>,
        entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult;
}

macro_rules! render_command_tuple_impl {
    ($(($name: ident, $view: ident, $entity: ident)),*) => {
        impl<P: PhaseItem, $($name: RenderCommand<P>),*> RenderCommand<P> for ($($name,)*) {
            type Param = ($($name::Param,)*);
            type ViewWorldQuery = ($($name::ViewWorldQuery,)*);
            type ItemWorldQuery = ($($name::ItemWorldQuery,)*);

            #[allow(non_snake_case)]
            fn render<'w>(
                _item: &P,
                ($($view,)*): ROQueryItem<'w, Self::ViewWorldQuery>,
                ($($entity,)*): ROQueryItem<'w, Self::ItemWorldQuery>,
                ($($name,)*): SystemParamItem<'w, '_, Self::Param>,
                _pass: &mut TrackedRenderPass<'w>,
            ) -> RenderCommandResult {
                $(if let RenderCommandResult::Failure = $name::render(_item, $view, $entity, $name, _pass) {
                    return RenderCommandResult::Failure;
                })*
                RenderCommandResult::Success
            }
        }
    };
}

all_tuples!(render_command_tuple_impl, 0, 15, C, V, E);

struct RenderCommandsInternal<P: PhaseItem> {
    render_commands: Vec<Box<dyn Command<P>>>,
    indices: HashMap<TypeId, RenderCommandId>,
}

/// A collection of all [`RenderCommands`] for the [`PhaseItem`] type.
///
/// To select the render command for each [`PhaseItem`] use the [`id`](Self::id) or
/// [`get_id`](Self::get_id) methods.
#[derive(Resource)]
pub struct RenderCommands<P: PhaseItem> {
    // TODO: can we avoid this RwLock?
    internal: RwLock<RenderCommandsInternal<P>>,
}

impl<P: PhaseItem> Default for RenderCommands<P> {
    fn default() -> Self {
        Self {
            internal: RwLock::new(RenderCommandsInternal {
                render_commands: Vec::new(),
                indices: HashMap::default(),
            }),
        }
    }
}

impl<P: PhaseItem> RenderCommands<P> {
    /// Retrieves the id of the corresponding [`RenderCommand`].
    pub fn get_id<C: RenderCommand<P>>(&self) -> Option<RenderCommandId> {
        self.internal
            .read()
            .indices
            .get(&TypeId::of::<C>())
            .copied()
    }

    /// Retrieves the id of the corresponding [`RenderCommand`].
    ///
    /// Fallible wrapper for [`Self::get_id()`]
    ///
    /// ## Panics
    /// If the id doesn't exist, this function will panic.
    pub fn id<C: RenderCommand<P>>(&self) -> RenderCommandId {
        self.get_id::<C>().unwrap_or_else(|| {
            panic!(
                "Render command {} not found for {}",
                std::any::type_name::<C>(),
                std::any::type_name::<P>()
            )
        })
    }

    /// Renders all items of the `render_phase` using their corresponding [`RenderCommand`].
    pub(crate) fn render<'w>(
        &self,
        world: &'w World,
        render_phase: &RenderPhase<P>,
        render_pass: &mut TrackedRenderPass<'w>,
        view: Entity,
    ) {
        let mut internal = self.internal.write();
        for render_command in &mut internal.render_commands {
            render_command.prepare(world);
        }

        for item in &render_phase.items {
            let render_command = &mut internal.render_commands[item.render_command_id().0 as usize];
            render_command.render(world, render_pass, view, item);
        }
    }

    /// Adds a [`RenderCommand`] (wrapped with a [`RenderCommandState`]) to this collection.
    fn add<C: RenderCommand<P>>(&self, render_command: Box<dyn Command<P>>) -> RenderCommandId {
        let mut internal = self.internal.write();
        let id = RenderCommandId(internal.render_commands.len() as u32);
        internal.render_commands.push(render_command);
        internal.indices.insert(TypeId::of::<C>(), id);
        id
    }
}

/// Registers a [`RenderCommand`] on the render app.
///
/// They are stored inside the [`RenderCommands`] resource of the app.
pub trait AddRenderCommand {
    /// Adds the [`RenderCommand`] for the specified [`PhaseItem`] type to the app.
    fn add_render_command<P: PhaseItem, C: RenderCommand<P>>(&mut self) -> &mut Self;
}

impl AddRenderCommand for App {
    fn add_render_command<P: PhaseItem, C: RenderCommand<P>>(&mut self) -> &mut Self {
        let render_command = RenderCommandState::<P, C>::initialize(&mut self.world);
        let render_commands = self
            .world
            .get_resource::<RenderCommands<P>>()
            .unwrap_or_else(|| {
                panic!(
                    "RenderCommands<{}> must be added to the world as a resource \
                     before adding render commands to it",
                    std::any::type_name::<P>(),
                );
            });
        render_commands.add::<C>(render_command);
        self
    }
}

// TODO: can we get rid of this trait entirely?
trait Command<P: PhaseItem>: Send + Sync + 'static {
    fn prepare(&mut self, world: &World);

    fn render<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    );
}

/// Wraps a [`RenderCommand`] into a state so that it can store system and query states to supply
/// the necessary data in the [`RenderCommand::render`] method.
///
/// The [`RenderCommand::Param`], [`RenderCommand::ViewWorldQuery`] and
/// [`RenderCommand::ItemWorldQuery`] are fetched from the ECS and passed to the command.
struct RenderCommandState<P: PhaseItem, C: RenderCommand<P>> {
    state: SystemState<C::Param>,
    view: QueryState<C::ViewWorldQuery>,
    entity: QueryState<C::ItemWorldQuery>,
}

impl<P: PhaseItem, C: RenderCommand<P>> RenderCommandState<P, C> {
    /// Creates a new [`RenderCommandState`] for the [`RenderCommand`].
    pub fn initialize(world: &mut World) -> Box<dyn Command<P>> {
        Box::new(Self {
            state: SystemState::new(world),
            view: world.query(),
            entity: world.query(),
        })
    }
}

impl<P: PhaseItem, C: RenderCommand<P>> Command<P> for RenderCommandState<P, C> {
    /// Prepares the render command to be used. This is called once and only once before the phase
    /// begins. There may be zero or more `draw` calls following a call to this function.
    fn prepare(&mut self, world: &'_ World) {
        self.state.update_archetypes(world);
        self.view.update_archetypes(world);
        self.entity.update_archetypes(world);
    }

    /// Fetches the ECS parameters for the wrapped [`RenderCommand`] and then,
    /// the phase item is rendered using this command.
    fn render<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    ) {
        let param = self.state.get_manual(world);
        let view = self.view.get_manual(world, view).unwrap();
        let entity = self.entity.get_manual(world, item.entity()).unwrap();
        // TODO: handle/log `RenderCommand` failure
        C::render(item, view, entity, param, pass);
    }
}
