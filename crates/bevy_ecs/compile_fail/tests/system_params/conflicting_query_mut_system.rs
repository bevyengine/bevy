//@error-in-other-file:  evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, &mut A>, bevy_ecs::system::Query<'_, '_, &mut A>))), {closure@tests/system_params/conflicting_query_mut_system.rs:20:37: 20:73}>::{constant#0}` failed
use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

#[derive(Component)]
pub struct C;

#[derive(Component)]
pub struct D;

#[derive(Component)]
pub struct E;

fn main() {
    Schedule::default().add_systems(|_: Query<&mut A>, _: Query<&mut A>| {});
}