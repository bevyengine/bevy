//@error-in-other-file: evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, &mut D, bevy_ecs::query::Or<(bevy_ecs::query::Or<(bevy_ecs::query::With<A>, bevy_ecs::query::With<B>)>, bevy_ecs::query::Or<(bevy_ecs::query::With<A>, bevy_ecs::query::With<C>)>)>>, bevy_ecs::system::Query<'_, '_, &mut D, bevy_ecs::query::Without<A>>))), {closure@tests/system_params/or_expanded_nested_or_with_and_disjoint_without.rs:20:37: 23:6}>::{constant#0}` failed
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
    Schedule::default().add_systems(|
        _: Query<&mut D, Or<(Or<(With<A>, With<B>)>, Or<(With<A>, With<C>)>)>>,
        _: Query<&mut D, Without<A>>,
    | {});
}