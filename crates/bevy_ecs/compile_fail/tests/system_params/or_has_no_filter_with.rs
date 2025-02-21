//@error-in-other-file: evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, &mut B, bevy_ecs::query::Or<(bevy_ecs::query::With<A>, bevy_ecs::query::With<B>)>>, bevy_ecs::system::Query<'_, '_, &mut B, bevy_ecs::query::Without<A>>))), {closure@tests/system_params/or_has_no_filter_with.rs:11:37: 11:109}>::{constant#0}` failed
use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

fn main() {
    Schedule::default().add_systems(|_: Query<&mut B, Or<(With<A>, With<B>)>>, _: Query<&mut B, Without<A>>| {});
}