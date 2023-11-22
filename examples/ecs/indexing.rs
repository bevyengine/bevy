//! This example demonstrates how to access `Component` data through an `Index`.
#![allow(clippy::type_complexity)]

use bevy::{
    hierarchy::{Index, IndexPlugin},
    prelude::*,
};
use std::hash::Hash;

#[derive(Component, Hash, Clone, PartialEq, Eq, Debug)]
struct Player(usize);

#[derive(Component)]
struct Head;

#[derive(Component)]
struct Body;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IndexPlugin::<Player>::default())
        .add_systems(Startup, |mut commands: Commands| {
            for player in 0..4 {
                commands.spawn((Head, Player(player)));
                commands.spawn((Body, Player(player)));
            }
        })
        .add_systems(FixedUpdate, get_bodies_for_head)
        .run();
}

fn get_bodies_for_head(
    heads: Query<(Entity, &Player), With<Head>>,
    bodies: Query<Entity, With<Body>>,
    mut index: Index<Player>,
) {
    for (head_entity, head_player) in heads.iter() {
        for body_entity in index.get(head_player).flat_map(|entity| bodies.get(entity)) {
            info!("{head_player:?}: {head_entity:?} <-> {body_entity:?}");
        }
    }
}
