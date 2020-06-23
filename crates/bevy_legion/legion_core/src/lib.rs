#![allow(dead_code)]

pub mod borrow;
pub mod command;
pub mod cons;
pub mod downcast_typename;
pub mod entity;
pub mod event;
pub mod filter;
pub mod guid_entity_allocator;
pub mod index;
pub mod iterator;
pub mod permission;
pub mod query;
pub mod storage;
pub mod subworld;
pub mod world;

#[cfg(feature = "serialize")]
pub mod serialize;

mod tuple;
mod zip;

pub mod prelude {
    pub use crate::{
        command::CommandBuffer,
        entity::Entity,
        event::Event,
        filter::filter_fns::*,
        query::{IntoQuery, Query as FilteredQuery, Read, Tagged, TryRead, TryWrite, Write},
        subworld::SubWorld,
        world::{EntityStore, Universe, World},
    };
}
