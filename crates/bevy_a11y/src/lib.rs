#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Reusable accessibility primitives
//!
//! This crate provides accessibility integration for the engine. It exposes the
//! [`AccessibilityPlugin`]. This plugin integrates `AccessKit`, a Rust crate
//! providing OS-agnostic accessibility primitives, with Bevy's ECS.
//!
//! ## Some notes on utility
//!
//! While this crate defines useful types for accessibility, it does not
//! actually power accessibility features in Bevy.
//!
//! Instead, it helps other interfaces coordinate their approach to
//! accessibility. Binary authors should add the [`AccessibilityPlugin`], while
//! library maintainers may use the [`AccessibilityRequested`] and
//! [`ManageAccessibilityUpdates`] resources.
//!
//! The [`AccessibilityNode`] component is useful in both cases. It helps
//! describe an entity in terms of its accessibility factors through an
//! `AccessKit` "node".
//!
//! Typical UI concepts, like buttons, checkboxes, and textboxes, are easily
//! described by this component, though, technically, it can represent any kind
//! of Bevy [`Entity`].
//!
//! ## This crate no longer re-exports `AccessKit`
//!
//! As of Bevy version 0.15, [the `accesskit` crate][accesskit_crate] is no
//! longer re-exported from this crate.[^accesskit_node_confusion] If you need
//! to use `AccessKit` yourself, you'll have to add it as a separate dependency
//! in your project's `Cargo.toml`.
//!
//! Make sure to use the same version of the `accesskit` crate as Bevy.
//! Otherwise, you may experience errors similar to: "Perhaps two different
//! versions of crate `accesskit` are being used?"
//!
//! [accesskit_crate]: https://crates.io/crates/accesskit
//! [`Entity`]: bevy_ecs::entity::Entity
//!
//! <!--
//! note: multi-line footnotes need to be indented like this!
//!
//! please do not remove the indentation, or the second paragraph will display
//! at the end of the module docs, **before** the footnotes...
//! -->
//!
//! [^accesskit_node_confusion]: Some users were confused about `AccessKit`'s
//!  `Node` type, sometimes thinking it was Bevy UI's primary way to define
//!  nodes!
//!
//!     For this reason, its re-export was removed by default. Users who need
//!     its types can instead manually depend on the `accesskit` crate.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use accesskit::Node;
use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, message::Message, resource::Resource, schedule::SystemSet};

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::reflect::ReflectResource, bevy_reflect::std_traits::ReflectDefault,
    bevy_reflect::Reflect,
};

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Wrapper struct for [`accesskit::ActionRequest`].
///
/// This newtype is required to use `ActionRequest` as a Bevy `Event`.
#[derive(Message, Deref, DerefMut)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct ActionRequest(pub accesskit::ActionRequest);

/// Tracks whether an assistive technology has requested accessibility
/// information.
///
/// This type is a [`Resource`] initialized by the
/// [`AccessibilityPlugin`]. It may be useful if a third-party plugin needs to
/// conditionally integrate with `AccessKit`.
///
/// In other words, this resource represents whether accessibility providers
/// are "turned on" or "turned off" across an entire Bevy `App`.
///
/// By default, it is set to `false`, indicating that nothing has requested
/// accessibility information yet.
///
/// [`Resource`]: bevy_ecs::resource::Resource
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Clone, Resource)
)]
pub struct AccessibilityRequested(Arc<AtomicBool>);

impl AccessibilityRequested {
    /// Checks if any assistive technology has requested accessibility
    /// information.
    ///
    /// If so, this method returns `true`, indicating that accessibility tree
    /// updates should be sent.
    pub fn get(&self) -> bool {
        self.load(Ordering::SeqCst)
    }

    /// Sets the app's preference for sending accessibility updates.
    ///
    /// If the `value` argument is `true`, this method requests that the app,
    /// including both Bevy and third-party interfaces, provides updates to
    /// accessibility information.
    ///
    /// Setting with `false` requests that the entire app stops providing these
    /// updates.
    pub fn set(&self, value: bool) {
        self.store(value, Ordering::SeqCst);
    }
}

/// Determines whether Bevy's ECS updates the accessibility tree.
///
/// This [`Resource`] tells Bevy internals whether it should be handling
/// `AccessKit` updates (`true`), or if something else is doing that (`false`).
///
/// It defaults to `true`. So, by default, Bevy is configured to maintain the
/// `AccessKit` tree.
///
/// Set to `false` in cases where an external GUI library is sending
/// accessibility updates instead. When this option is set inconsistently with
/// that requirement, the external library and ECS will generate conflicting
/// updates.
///
/// [`Resource`]: bevy_ecs::resource::Resource
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Clone, Default)
)]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
pub struct ManageAccessibilityUpdates(bool);

impl Default for ManageAccessibilityUpdates {
    fn default() -> Self {
        Self(true)
    }
}

impl ManageAccessibilityUpdates {
    /// Returns `true` if Bevy's ECS should update the accessibility tree.
    pub fn get(&self) -> bool {
        self.0
    }

    /// Sets whether Bevy's ECS should update the accessibility tree.
    pub fn set(&mut self, value: bool) {
        self.0 = value;
    }
}

/// Represents an entity to `AccessKit` through an [`accesskit::Node`].
///
/// Platform-specific accessibility APIs utilize `AccessKit` nodes in their
/// accessibility frameworks. So, this component acts as a translation between
/// "Bevy entity" and "platform-agnostic accessibility element".
///
/// ## Organization in the `AccessKit` Accessibility Tree
///
/// `AccessKit` allows users to form a "tree of nodes" providing accessibility
/// information. That tree is **not** Bevy's ECS!
///
/// To explain, let's say this component is added to an entity, `E`.
///
/// ### Parent and Child
///
/// If `E` has a parent, `P`, and `P` also has this `AccessibilityNode`
/// component, then `E`'s `AccessKit` node will be a child of `P`'s `AccessKit`
/// node.
///
/// Resulting `AccessKit` tree:
/// - P
///     - E
///
/// In other words, parent-child relationships are maintained, but only if both
/// have this component.
///
/// ### On the Window
///
/// If `E` doesn't have a parent, or if the immediate parent doesn't have an
/// `AccessibilityNode`, its `AccessKit` node will be an immediate child of the
/// primary window.
///
/// Resulting `AccessKit` tree:
/// - Primary window
///     - E
///
/// When there's no `AccessKit`-compatible parent, the child lacks hierarchical
/// information in `AccessKit`. As such, it is placed directly under the
/// primary window on the `AccessKit` tree.
///
/// This behavior may or may not be intended, so please utilize
/// `AccessibilityNode`s with care.
#[derive(Component, Clone, Deref, DerefMut)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct AccessibilityNode(
    /// A representation of this component's entity to `AccessKit`.
    ///
    /// Note that, with its parent struct acting as just a newtype, users are
    /// intended to directly update this field.
    pub Node,
);

impl From<Node> for AccessibilityNode {
    /// Converts an [`accesskit::Node`] into the Bevy Engine
    /// [`AccessibilityNode`] newtype.
    ///
    /// Doing so allows it to be inserted onto Bevy entities, representing Bevy
    /// entities in the `AccessKit` tree.
    fn from(node: Node) -> Self {
        Self(node)
    }
}

/// A system set relating to accessibility.
///
/// Helps run accessibility updates all at once.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize, Clone)
)]
pub enum AccessibilitySystems {
    /// Update the accessibility tree.
    Update,
}

/// Deprecated alias for [`AccessibilitySystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `AccessibilitySystems`.")]
pub type AccessibilitySystem = AccessibilitySystems;

/// Plugin managing integration with accessibility APIs.
///
/// Note that it doesn't handle GUI aspects of this integration, instead
/// providing helpful resources for other interfaces to utilize.
///
/// ## Behavior
///
/// This plugin's main role is to initialize the [`AccessibilityRequested`] and
/// [`ManageAccessibilityUpdates`] resources to their default values, meaning:
///
/// - no assistive technologies have requested accessibility information yet,
///   and
/// - Bevy's ECS will manage updates to the accessibility tree.
#[derive(Default)]
pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<AccessibilityRequested>()
            .init_resource::<ManageAccessibilityUpdates>()
            .allow_ambiguous_component::<AccessibilityNode>();
    }
}
