//! Reusable `bevy_ecs` macro logic. This enables defining derives that internally derive ECS traits
//! like `Component`.

/// `Component` macro logic. The primary interface is [`DeriveComponent`](component::DeriveComponent).
pub mod component;

/// `MapEntities` macro logic.
pub mod map_entities;
