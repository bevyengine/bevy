//! This example demonstrates how to access `Component` data through an `Index`.
#![allow(clippy::type_complexity)]

use bevy::{
    ecs::{indexing::*, query::ReadOnlyWorldQuery},
    prelude::*,
};
use std::{hash::Hash, marker::PhantomData};

pub struct IndexPlugin<T, F = (), I = SimpleIndexer<T>>(PhantomData<fn(T, F, I)>);

impl<T, F, I> Default for IndexPlugin<T, F, I> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, I, F> Plugin for IndexPlugin<T, F, I>
where
    T: Component,
    I: Indexer<Input = T> + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<IndexBacking<T, F, I>>()
            .add_systems(Update, Index::<T, F, I>::update_index);
    }
}

// Usage

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
