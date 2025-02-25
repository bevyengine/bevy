//@error-in-other-file: evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, &mut E, (bevy_ecs::query::Or<((bevy_ecs::query::With<B>, bevy_ecs::query::With<C>), (bevy_ecs::query::With<C>, bevy_ecs::query::With<D>))>, bevy_ecs::query::With<A>)>, bevy_ecs::system::Query<'_, '_, &mut E, bevy_ecs::query::Without<D>>))), {closure@tests/system_params/or_expanded_nested_with_and_disjoint_without.rs:20:37: 22:38}>::{constant#0}` failed
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
        _: Query<&mut E, (Or<((With<B>, With<C>), (With<C>, With<D>))>, With<A>)>,
        _: Query<&mut E, Without<D>>| {});
}