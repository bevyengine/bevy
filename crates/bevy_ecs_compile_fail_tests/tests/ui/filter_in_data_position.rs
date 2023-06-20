use bevy_ecs::prelude::*;

#[derive(Component)]
struct A;

fn system(mut query: Query<With<A>>) {
    
}

fn main() {}