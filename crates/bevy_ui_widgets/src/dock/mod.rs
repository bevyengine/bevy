//! Dock layouts for Bevy UI.
//!
//! A [`DockTree`] describes how a region of the screen is divided into tab groups. It is
//! plain data: layout changes such as moving, splitting or closing tabs are made on the tree,
//! and [`DockPlugin`] builds and updates UI to match.
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
//! The tree is the source of truth for the layout, and the dock's UI entities are derived from
//! it. Widgets such as tabs and splitters only propose changes, which are applied to the tree,
//! and the UI is then brought back in line with it. Keeping the layout as plain data makes
//! operations like splitting at an edge or collapsing empty groups simple to write and test,
//! makes layouts easy to save and restore, and lets the UI entities be rebuilt without losing
//! the layout.
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
//! The above code produces this layout, where `O` is the outliner, `V` the viewport, `I/S` the
//! inspector and settings tabs, and `A` the assets:
//!
//! ```text
//! +---+---+-----+
//! | O | V | I/S |
//! |   +---+     |
//! |   | A |     |
//! +---+---+-----+
//! ```
//!
//! # UI
//!
//! [`DockPlugin`] is part of [`UiWidgetsPlugins`](crate::UiWidgetsPlugins). With it, an
//! entity with a [`DockTree`] and a [`Node`] gets UI built as its children: each split becomes a
//! [`SplitPane`] with [`SplitPaneHandle`]s between its panes, and each leaf becomes a column
//! holding a [`TabList`] and a content area. The app gives each [`DockPanelKey`] a title and a
//! content builder in [`DockPanels`]. Content is built once per tab and kept until the tab is
//! removed, so its state survives switching and moving tabs. The UI is headless: it has the
//! layout it needs and no colors.
//!
//! The UI is controlled. Selecting a tab or dragging a splitter sends a [`DockChange`] to
//! the dock entity instead of changing the tree. Add [`dock_self_update`] as an observer on
//! the dock entity to apply these changes, or observe [`DockChange`] and edit the tree with
//! [`DockEdit::apply`] or other operations.
//!
//! [`ControlOrientation`]: crate::ControlOrientation
//! [`Node`]: bevy_ui::Node
//! [`SplitPane`]: crate::SplitPane
//! [`SplitPaneHandle`]: crate::SplitPaneHandle
//! [`TabList`]: crate::TabList
//! [`Pane::size`]: crate::Pane::size

mod tree;
mod ui;

pub use tree::*;
pub use ui::*;
