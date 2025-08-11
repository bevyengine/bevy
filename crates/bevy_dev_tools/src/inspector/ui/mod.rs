//! Reusable UI components for the remote inspector
//!
//! This module provides a comprehensive set of UI components for building interactive
//! entity and component inspectors. The components are designed with reusability in mind
//! and could potentially be upstreamed to `bevy_ui` in the future.
//!
//! ## Components
//!
//! - **Entity List**: High-performance virtual scrolling list for displaying thousands of entities
//! - **Component Viewer**: Interactive component data display with live updates and text selection
//! - **Connection Status**: Real-time connection status indicator for remote applications
//! - **Collapsible Sections**: Expandable/collapsible content areas for organizing data
//! - **Virtual Scrolling**: Performance-optimized scrolling system for large datasets

pub mod collapsible_section;
pub mod component_viewer;
pub mod connection_status;
pub mod entity_list;
pub mod virtual_scrolling;

pub use collapsible_section::*;
pub use component_viewer::*;
pub use connection_status::*;
pub use entity_list::*;
pub use virtual_scrolling::*;
