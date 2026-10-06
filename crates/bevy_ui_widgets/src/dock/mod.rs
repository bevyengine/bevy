//! Dock layouts for Bevy UI, described as plain data.
//!
//! A [`DockTree`] describes how a region of the screen is divided into tab groups. It holds
//! no entities and draws nothing; UI is built from it and kept in sync by other code. Layout
//! changes such as moving, splitting or closing tabs are made on the tree.
//!
//! The tree has two kinds of node:
//!
//! - a leaf, [`DockLeaf`], is a tab group: an ordered list of [`DockTab`]s and the one that
//!   is active
//! - a split, [`DockSplit`], lays out two or more children along a [`ControlOrientation`],
//!   each with a flex weight that matches [`Pane::size`]
//!
//! Each tab names its content with a [`DockPanelKey`]. The app decides what content a key
//! stands for, so the same tree can be saved and loaded across runs.
//!
//! [`DockNodeId`]s of leaves and [`DockTabId`]s are stable across edits and never reused.
//! Split nodes are created and removed as the tree is simplified: empty leaves are removed,
//! splits with a single child are replaced by that child, and a split nested in a split of
//! the same orientation is merged into its parent.
//!
//! [`DockTree`] is a component, so an app can have several docks, one per root entity. With
//! the `serialize` feature it can be saved and loaded with serde.
//!
//! ```
//! use bevy_ui_widgets::{DockEdge, DockTree};
//!
//! let mut tree = DockTree::new();
//! let center = tree.root();
//! tree.add_tab(center, "viewport").unwrap();
//! tree.split(center, DockEdge::Left, "outliner").unwrap();
//! let (right, _) = tree.split(center, DockEdge::Right, "inspector").unwrap();
//! tree.add_tab(right, "settings").unwrap();
//! tree.split(center, DockEdge::Bottom, "assets").unwrap();
//!
//! assert_eq!(tree.leaves().count(), 4);
//! ```
//!
//! [`ControlOrientation`]: crate::ControlOrientation
//! [`Pane::size`]: crate::Pane::size

mod tree;

pub use tree::*;
