#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Accessibility for Bevy

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub use accesskit;
use accesskit::NodeBuilder;
use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity, Event},
    schedule::SystemSet,
    system::Resource,
};

/// Wrapper struct for [`accesskit::ActionRequest`]. Required to allow it to be used as an `Event`.
#[derive(Event, Deref, DerefMut)]
pub struct ActionRequest(pub accesskit::ActionRequest);

/// Resource that tracks whether an assistive technology has requested
/// accessibility information.
///
/// Useful if a third-party plugin needs to conditionally integrate with
/// `AccessKit`
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut)]
pub struct AccessibilityRequested(Arc<AtomicBool>);

impl AccessibilityRequested {
    /// Returns `true` if an access technology is active and accessibility tree
    /// updates should be sent.
    pub fn get(&self) -> bool {
        self.load(Ordering::SeqCst)
    }

    /// Sets whether accessibility updates were requested by an access technology.
    pub fn set(&self, value: bool) {
        self.store(value, Ordering::SeqCst);
    }
}

/// Resource whose value determines whether the accessibility tree is updated
/// via the ECS.
///
/// Set to `false` in cases where an external GUI library is sending
/// accessibility updates instead. Without this, the external library and ECS
/// will generate conflicting updates.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct ManageAccessibilityUpdates(bool);

impl Default for ManageAccessibilityUpdates {
    fn default() -> Self {
        Self(true)
    }
}

impl ManageAccessibilityUpdates {
    /// Returns `true` if the ECS should update the accessibility tree.
    pub fn get(&self) -> bool {
        self.0
    }

    /// Sets whether the ECS should update the accessibility tree.
    pub fn set(&mut self, value: bool) {
        self.0 = value;
    }
}

/// Component to wrap a [`accesskit::Node`], representing this entity to the platform's
/// accessibility API.
///
/// If an entity has a parent, and that parent also has an `AccessibilityNode`,
/// the entity's node will be a child of the parent's node.
///
/// If the entity doesn't have a parent, or if the immediate parent doesn't have
/// an `AccessibilityNode`, its node will be an immediate child of the primary window.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct AccessibilityNode(pub NodeBuilder);

impl From<NodeBuilder> for AccessibilityNode {
    fn from(node: NodeBuilder) -> Self {
        Self(node)
    }
}

/// Resource representing which entity has keyboard focus, if any.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Focus(pub Option<Entity>);

/// Set enum for the systems relating to accessibility
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum AccessibilitySystem {
    /// Update the accessibility tree
    Update,
}

/// Plugin managing non-GUI aspects of integrating with accessibility APIs.
pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<AccessibilityRequested>()
            .init_resource::<ManageAccessibilityUpdates>()
            .init_resource::<Focus>()
            .allow_ambiguous_component::<AccessibilityNode>();
    }
}
