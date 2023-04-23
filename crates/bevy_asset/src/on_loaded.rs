// TODO: this is a very rough experiment. Ideally this uses CombinatorSystem for simplicity and safety but that isn't expressable to my knowledge
// See the array_texture.rs example for usage

use crate::{Asset, AssetEvent, AssetServer, Handle};
use bevy_ecs::{
    archetype::ArchetypeComponentId, component::ComponentId, event::ManualEventReader, prelude::*,
    query::Access,
};
use std::any::TypeId;

// pub fn on_loaded<'a, A: Asset + FromWorld + Clone, Marker, S: IntoSystem<A, (), Marker>>(
//     system: S,
// ) -> CombinatorSystem<OnLoaded, _, S::System> {
//     on_loaded_helper(|asset: Local<A>, reader: EventReader<Events<AssetEvent>>|, systmt)
// }

// pub fn on_loaded_helper<
//     'a,
//     A: Asset,
//     S1: IntoSystem<(), Option<A>, Marker1>,
//     S2: IntoSystem<A, (), Marker2>,
//     Marker1,
//     Marker2,
// >(
//     system1: S1,
//     system2: S2,
// ) -> CombinatorSystem<OnLoaded, S1::System, S2::System> {
//     CombinatorSystem::new(system1, system2, "HI".into())
// }

// pub struct OnLoaded;

// impl<A, B> Combine<A, B> for OnLoaded
// where
//     A: System,
//     B: System<In = A::Out>,
// {
//     type In = A::In;
//     type Out = B::Out;

//     fn combine(
//         input: Self::In,
//         a: impl FnOnce(A::In) -> A::Out,
//         b: impl FnOnce(B::In) -> B::Out,
//     ) -> Self::Out {
//         let value = a(input);
//         b(value)
//     }
// }

pub fn on_loaded<'a, A: Asset + FromWorld + Clone, Marker, S: IntoSystem<A, (), Marker>>(
    system: S,
) -> OnLoadedSystem<A, S, Marker> {
    OnLoadedSystem {
        asset: None,
        handle: None,
        loaded: false,
        archetype_component_access: Default::default(),
        component_access: Default::default(),
        event_reader: ManualEventReader::default(),
        system: IntoSystem::into_system(system),
    }
}

pub struct OnLoadedSystem<A: Asset + FromWorld, S: IntoSystem<A, (), Marker>, Marker> {
    asset: Option<A>,
    handle: Option<Handle<A>>,
    event_reader: ManualEventReader<AssetEvent<A>>,
    archetype_component_access: Access<ArchetypeComponentId>,
    component_access: Access<ComponentId>,
    loaded: bool,
    system: S::System,
}

impl<A: Asset + FromWorld + Clone, S: IntoSystem<A, (), Marker> + 'static, Marker: 'static> System
    for OnLoadedSystem<A, S, Marker>
{
    type In = ();

    type Out = ();

    fn name(&self) -> std::borrow::Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }

    fn type_id(&self) -> std::any::TypeId {
        TypeId::of::<Self>()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        self.system.is_send()
    }

    fn is_exclusive(&self) -> bool {
        false
    }

    unsafe fn run_unsafe(&mut self, _input: Self::In, world: &World) -> Self::Out {
        if self.loaded {
            return;
        }
        for event in self
            .event_reader
            .iter(world.resource::<Events<AssetEvent<A>>>())
        {
            if event.is_loaded_with_dependencies(self.handle.as_ref().unwrap()) {
                self.loaded = true;
                self.system.run_unsafe(self.asset.take().unwrap(), world);
            }
        }
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        let asset = A::from_world(world);
        let handle = world.resource::<AssetServer>().load_asset(asset.clone());
        self.handle = Some(handle);
        self.asset = Some(asset);
        // TODO: This cannot be expressed without making these methods public. Ideally this system is
        // implemented using CombinatorSystem
        bevy_log::error!("on_loaded is currently unsafe to use because it fails to encode read access to asset events");
        // let component_id = world.initialize_resource::<Events<AssetEvent>>();
        // self.component_access.add_read(component_id);

        // let archetype_component_id = world
        //     .get_resource_archetype_component_id(component_id)
        //     .unwrap();
        // self.archetype_component_access
        //     .add_read(archetype_component_id);

        self.system.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.system.update_archetype_component_access(world);
        self.archetype_component_access
            .extend(self.system.archetype_component_access())
    }

    fn check_change_tick(&mut self, change_tick: bevy_ecs::component::Tick) {
        self.system.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> bevy_ecs::component::Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: bevy_ecs::component::Tick) {
        self.system.set_last_run(last_run)
    }
}
