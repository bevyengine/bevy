//@error-in-other-file: evaluation of `bevy_ecs::schedule::Schedule::add_systems::<(bevy_ecs::schedule::Infallible, (bevy_ecs::system::IsFunctionSystem, fn(bevy_ecs::system::Query<'_, '_, bevy_ecs::query::AnyOf<(&mut A, &A)>>))), {closure@tests/system_params/any_of_with_mut_and_ref.rs:8:37: 8:68}>::{constant#0}` failed
use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct A;

fn main() {
    Schedule::default().add_systems(|_: Query<AnyOf<(&mut A, &A)>>| {});
}