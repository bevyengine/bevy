use bevy_reflect::Reflect;

use super::reflect_query_structs::{EntityQuerydyn, MutQuerydyn, Querydyn, RefQuerydyn};
use crate::{
    change_detection::{Mut, Ref},
    component::Component,
    entity::Entity,
    query::QuerySingleError,
    query::With,
    world::EntityRef,
    world::World,
};

pub(super) type SingleResult<T> = Result<T, QuerySingleError>;

pub(super) fn reflect_ref<C: Component + Reflect>(entity: EntityRef) -> Option<Ref<dyn Reflect>> {
    let component = entity.get_ref::<C>()?;
    Some(component.map(C::as_reflect))
}
pub(super) fn get_single<C: Component + Reflect>(world: &mut World) -> SingleResult<&dyn Reflect> {
    let component = world.query::<&C>().get_single(world)?;
    Ok(component.as_reflect())
}
pub(super) fn get_single_ref<C: Component + Reflect>(
    world: &mut World,
) -> SingleResult<Ref<dyn Reflect>> {
    let component = world.query::<Ref<C>>().get_single(world)?;
    Ok(component.map(C::as_reflect))
}
pub(super) fn get_single_mut<C: Component + Reflect>(
    world: &mut World,
) -> SingleResult<Mut<dyn Reflect>> {
    let query = world.query::<&mut C>().get_single_mut(world);
    Ok(query?.map_unchanged(C::as_reflect_mut))
}
pub(super) fn get_single_entity<C: Component + Reflect>(world: &mut World) -> SingleResult<Entity> {
    world.query_filtered::<Entity, With<C>>().get_single(world)
}
pub(super) fn query<C: Component + Reflect>(world: &mut World) -> Querydyn {
    Querydyn(Box::new(world.query::<&C>()))
}
pub(super) fn query_mut<C: Component + Reflect>(world: &mut World) -> MutQuerydyn {
    MutQuerydyn(Box::new(world.query::<&mut C>()))
}
pub(super) fn query_ref<C: Component + Reflect>(world: &mut World) -> RefQuerydyn {
    RefQuerydyn(Box::new(world.query::<Ref<C>>()))
}
pub(super) fn query_entities<C: Component + Reflect>(world: &mut World) -> EntityQuerydyn {
    EntityQuerydyn(Box::new(world.query_filtered::<Entity, With<C>>()))
}
