//! # Bevy Editor
//!
//! A comprehensive, modular real-time editor for the Bevy game engine using bevy_remote.
//! 
//! ## Features
//! 
//! - **Entity Inspector**: Browse and select entities in a clean interface
//! - **Component Inspector**: Detailed component viewing with expandable fields
//! - **Modern UI**: Dark theme with professional styling
//! - **Remote Integration**: Built-in support for `bevy_remote` protocol
//! - **Modular Design**: Use individual components or the full editor
//! - **Scrollable Views**: Native Bevy scrolling with bevy_core_widgets integration
//! 
//! ## Quick Start
//! 
//! ```rust
//! use bevy::prelude::*;
//! use bevy_editor::prelude::EditorPlugin;
//! 
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EditorPlugin)
//!         .run();
//! }
//! ```
//! 
//! ## Architecture
//! 
//! The editor is built with a modular architecture that allows for flexible usage:
//! 
//! - **Core Editor**: Main editor plugin that orchestrates all components
//! - **Panels**: Individual UI panels (entity list, component inspector)
//! - **Widgets**: Reusable UI components (scroll views, expansion buttons)
//! - **Remote Client**: HTTP client for bevy_remote protocol
//! - **Formatting**: Component data formatting and display utilities
//! - **Themes**: UI styling and theming system
//! 
//! ## Modular Usage
//! 
//! Individual components can be used separately for custom editor implementations:
//! 
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::panels::EntityListPlugin;
//! use bevy_editor::widgets::WidgetsPlugin;
//! 
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins((EntityListPlugin, WidgetsPlugin))
//!         .run();
//! }
//! ```

pub mod editor;
pub mod widgets;
pub mod panels;
pub mod remote;
pub mod formatting;
pub mod themes;
pub mod live_editor;

/// Convenient re-exports for common editor functionality.
pub mod prelude {
    //! Common imports for bevy_editor usage.
    
    // Main plugins
    pub use crate::inspector::editor::EditorPlugin;
    pub use crate::inspector::live_editor::LiveEditorPlugin;
    
    // Individual plugins for modular usage
    pub use crate::inspector::panels::{EntityListPlugin, ComponentInspectorPlugin};
    pub use crate::inspector::widgets::WidgetsPlugin;
    pub use crate::inspector::remote::RemoteClientPlugin;
    
    // Core types for remote connection
    pub use crate::inspector::remote::types::{
        RemoteEntity, ConnectionStatus, EntitiesFetched, ComponentDataFetched
    };
    
    // Widget builders for custom implementations
    pub use crate::inspector::widgets::{ScrollViewBuilder, CoreScrollArea, ScrollContent};
    
    // Theme system
    pub use crate::inspector::widgets::EditorTheme;
}
