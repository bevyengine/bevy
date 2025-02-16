//@error-in-other-file: evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, &mut B, bevy_ecs::query::With<A>>, bevy_ecs::system::Query<'_, '_, &mut B, bevy_ecs::query::Or<((), bevy_ecs::query::Without<A>)>>))), {closure@tests/system_params/with_and_disjoint_or_empty_without.rs:20:37: 20:104}>::{constant#0}` failed
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
    Schedule::default().add_systems(|_: Query<&mut B, With<A>>, _: Query<&mut B, Or<((), Without<A>)>>| {});
}