#![allow(dead_code)]

pub mod borrow;
pub mod command;
pub mod cons;
pub mod entity;
pub mod event;
pub mod filter;
pub mod index;
pub mod iterator;
pub mod query;
pub mod storage;
pub mod world;
pub mod downcast_typename;
pub mod guid_entity_allocator;

#[cfg(feature = "serialize")]
pub mod serialize;

mod system_fn_types;
mod tuple;
mod zip;

pub mod prelude {
    pub use crate::{
        borrow::{Ref as Com, RefMut as ComMut},
        command::CommandBuffer,
        entity::Entity,
        event::Event,
        filter::filter_fns::*,
        query::{IntoQuery, Query as FilteredQuery, Read, Tagged, TryRead, TryWrite, Write},
        world::{Universe, World},
    };
}
