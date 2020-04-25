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
        query::{IntoQuery, Query, Read, Tagged, TryRead, TryWrite, Write},
        world::{Universe, World},
    };
}
