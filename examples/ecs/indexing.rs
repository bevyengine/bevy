//! This example demonstrates how to access `Component` data through an `Index`.
#![allow(clippy::type_complexity)]

use bevy::{
    ecs::{
        component::Tick,
        system::{SystemChangeTick, SystemParam},
    },
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_internal::ecs::query::ReadOnlyWorldQuery;
use std::{hash::Hash, marker::PhantomData};

pub trait Indexer {
    type Input: Component;
    type Index: Hash + Eq + Clone + 'static;

    fn index(input: &Self::Input) -> Self::Index;
}

pub struct SimpleIndexer<T>(PhantomData<T>);

impl<T> Indexer for SimpleIndexer<T> where T: Component + Hash + Eq + Clone {
    type Input = T;

    type Index = T;

    fn index(input: &Self::Input) -> Self::Index {
        input.clone()
    }
}

#[derive(Resource)]
struct IndexBacking<T, F = (), I = SimpleIndexer<T>> {
    forward: HashMap<T, HashSet<Entity>>,
    reverse: HashMap<Entity, T>,
    last_this_run: Option<Tick>,
    _phantom: PhantomData<fn(F, I)>,
}

impl<T, F, I> Default for IndexBacking<T, F, I> {
    fn default() -> Self {
        Self {
            forward: default(),
            reverse: default(),
            last_this_run: default(),
            _phantom: PhantomData,
        }
    }
}

impl<T, F> IndexBacking<T, F> {
    fn update(&mut self, entity: Entity, value: Option<T>) -> Option<T>
    where
        T: Hash + Eq + Clone,
    {
        let old = if let Some(ref value) = value {
            self.reverse.insert(entity, value.clone())
        } else {
            self.reverse.remove(&entity)
        };

        if let Some(ref old) = old {
            if let Some(set) = self.forward.get_mut(old) {
                set.remove(&entity);

                if set.is_empty() {
                    self.forward.remove(old);
                }
            }
        }

        if let Some(value) = value {
            self.forward.entry(value).or_default().insert(entity);
        };

        old
    }

    fn get(&self, value: &T) -> Option<impl Iterator<Item = Entity> + '_>
    where
        T: Hash + Eq + Clone,
    {
        Some(self.forward.get(value)?.iter().copied())
    }
}

#[derive(SystemParam)]
pub struct Index<'w, 's, T, F = ()>
where
    T: Component + Hash + Eq,
    F: ReadOnlyWorldQuery + 'static,
{
    changed: Query<'w, 's, (Entity, Ref<'static, T>), (Changed<T>, F)>,
    removed: RemovedComponents<'w, 's, T>,
    index: ResMut<'w, IndexBacking<T, F>>,
    this_run: SystemChangeTick,
}

impl<'w, 's, T, F> Index<'w, 's, T, F>
where
    T: Component + Hash + Eq + Clone,
    F: ReadOnlyWorldQuery + 'static,
{
    fn update_index_internal(&mut self) {
        let this_run = self.this_run.this_run();

        // Remove old entires
        for entity in self.removed.read() {
            self.index.update(entity, None);
        }

        // Update new and existing entries
        for (entity, component) in self.changed.iter() {
            self.index.update(entity, Some(component.clone()));
        }

        self.index.last_this_run = Some(this_run);
    }

    fn update_index(mut index: Index<T>) {
        index.update_index_internal();
    }

    fn ensure_updated(&mut self) {
        let this_run = self.this_run.this_run();

        if self.index.last_this_run != Some(this_run) {
            self.update_index_internal();
        }
    }

    pub fn get(&mut self, value: &T) -> Option<impl Iterator<Item = Entity> + '_> {
        self.ensure_updated();

        self.index.get(value)
    }

    pub fn iter(&mut self) -> impl Iterator<Item = Entity> + '_ {
        self.ensure_updated();

        self.index.reverse.keys().copied()
    }
}

pub struct IndexPlugin<T>(PhantomData<T>);

impl<T> Default for IndexPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> Plugin for IndexPlugin<T> where T: Component + Hash + Eq + Clone {
    fn build(&self, app: &mut App) {
        app.init_resource::<IndexBacking<T>>()
            .add_systems(Update, Index::<T>::update_index);
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
    mut index: Index<Player>
) {
    for (head_entity, head_player) in heads.iter() {
        let Some(body_entities) = index.get(head_player) else {
            continue;
        };

        for body_entity in body_entities {
            let Ok(body_entity) = bodies.get(body_entity) else {
                continue;
            };

            info!("{head_player:?}: {head_entity:?} <-> {body_entity:?}");
        }
    }
}
