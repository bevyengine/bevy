use crate::render_phase::TrackedRenderPass;
use bevy_ecs::{entity::Entity, world::World};
use bevy_utils::HashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{any::TypeId, fmt::Debug, hash::Hash};

// TODO: should this be generic on "drawn thing"? would provide more flexibility and  explicitness
// instead of hard coded draw key and sort key
pub trait Draw: Send + Sync + 'static {
    fn draw(
        &mut self,
        world: &World,
        pass: &mut TrackedRenderPass,
        view: Entity,
        draw_key: usize,
        sort_key: usize,
    );
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DrawFunctionId(usize);

#[derive(Default)]
pub struct DrawFunctionsInternal {
    pub draw_functions: Vec<Box<dyn Draw>>,
    pub indices: HashMap<TypeId, DrawFunctionId>,
}

impl DrawFunctionsInternal {
    pub fn add<D: Draw>(&mut self, draw_function: D) -> DrawFunctionId {
        self.draw_functions.push(Box::new(draw_function));
        let id = DrawFunctionId(self.draw_functions.len() - 1);
        self.indices.insert(TypeId::of::<D>(), id);
        id
    }

    pub fn get_mut(&mut self, id: DrawFunctionId) -> Option<&mut dyn Draw> {
        self.draw_functions.get_mut(id.0).map(|f| &mut **f)
    }

    pub fn get_id<D: Draw>(&self) -> Option<DrawFunctionId> {
        self.indices.get(&TypeId::of::<D>()).copied()
    }
}

#[derive(Default)]
pub struct DrawFunctions {
    internal: RwLock<DrawFunctionsInternal>,
}

impl DrawFunctions {
    pub fn read(&self) -> RwLockReadGuard<'_, DrawFunctionsInternal> {
        self.internal.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, DrawFunctionsInternal> {
        self.internal.write()
    }
}
