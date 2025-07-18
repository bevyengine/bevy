//! # Remote Connection System
//!
//! This module provides comprehensive remote connection management for
//! communicating with bevy_remote servers. It handles HTTP communication,
//! connection state management, and data serialization/deserialization.
//!
//! ## Components
//!
//! - **Client**: HTTP client for bevy_remote protocol communication
//! - **Types**: Data structures for entities, components, and events
//! - **Connection**: Connection state management and auto-reconnection
//!
//! ## Protocol Support
//!
//! The remote system supports the bevy_remote protocol:
//! - Entity queries with component filtering
//! - Component data fetching and parsing
//! - Real-time connection status monitoring
//! - Automatic reconnection on connection loss
//!
//! ## Usage
//!
//! The remote client can be used standalone or as part of the editor:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::remote::RemoteClientPlugin;
//!
//! App::new()
//!     .add_plugins(RemoteClientPlugin)
//!     .run();
//! ```

pub mod types;
pub mod client;
pub mod connection;

use bevy::prelude::*;
pub use types::*;
pub use client::*;
pub use connection::*;

/// Plugin that handles remote connection functionality
#[derive(Default)]
pub struct RemoteClientPlugin;

impl Plugin for RemoteClientPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_remote_connection)
            .add_observer(handle_entities_fetched)
            .init_resource::<EditorState>()
            .init_resource::<RemoteConnection>();
    }
}
