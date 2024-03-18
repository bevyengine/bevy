//! This crate provides additional utilities for the [Bevy game engine](https://bevyengine.org),
//! focused on improving developer experience.
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::fmt::Debug;
use bevy_app::prelude::*;
use bevy_ecs::system::Resource;
use bevy_utils::HashMap;
use uuid::Uuid;

#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;
pub mod fps_overlay;

/// Enables developer tools in an [`App`]. This plugin is added automatically with `bevy_dev_tools`
/// feature.
///
/// Warning: It is not recommended to enable this in final shipped games or applications.
/// Dev tools provide a high level of access to the internals of your application,
/// and may interfere with ordinary use and gameplay.
///
/// To enable developer tools, you can either:
///
/// - Create a custom crate feature (e.g "`dev_mode`"), which enables the `bevy_dev_tools` feature
/// along with any other development tools you might be using:
///
/// ```toml
/// [feature]
/// dev_mode = ["bevy/bevy_dev_tools", "other_dev_tools"]
/// ```
///
/// - Use `--feature bevy/bevy_dev_tools` flag when using the `cargo run` command:
///
/// `cargo run --features bevy/bevy_dev_tools`
///
/// - Add the `bevy_dev_tools` feature to the bevy dependency in your `Cargo.toml` file:
///
/// `features = ["bevy_dev_tools"]`
///
///  Note: The third method is not recommended, as it requires you to remove the feature before
///  creating a build for release to the public.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(feature = "bevy_ci_testing")]
        {
            ci_testing::setup_app(_app);
        }
    }
}

/// Unique identifier for [`DevTool`].
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct DevToolId(pub Uuid);

impl DevToolId {
    /// Creates a [`DevToolId`] from u128.
    pub const fn from_u128(value: u128) -> DevToolId {
        DevToolId(Uuid::from_u128(value))
    }
}

/// Trait implemented for every dev tool.
pub trait DevTool: Sync + Send + Debug + 'static {}

/// Information about dev tool.
#[derive(Debug)]
pub struct DevToolConfig {
    /// Identifier of a dev tool.
    pub id: DevToolId,
    /// Tool specific configuration.
    pub tool_config: Box<dyn DevTool>,
    is_enabled: bool,
}

impl DevToolConfig {
    /// Returns true if [`DevTool`] is enabled.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Enables [`DevTool`].
    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    /// Disables
    pub fn disable(&mut self) {
        self.is_enabled = false;
    }

    /// Toggles [`DevTool`].
    pub fn toggle(&mut self) {
        self.is_enabled = !self.is_enabled;
    }
}

impl DevToolConfig {
    /// Creates a new [`DevTool`] from a specified [`DevToolId`].
    /// New tool is enabled by default.
    pub fn new(id: DevToolId, tool_config: impl DevTool) -> DevToolConfig {
        DevToolConfig {
            id,
            tool_config: Box::new(tool_config),
            is_enabled: true,
        }
    }
}

/// A collection of [`DevTool`]s.
#[derive(Resource, Default)]
pub struct DevToolsStore {
    dev_tools: HashMap<DevToolId, DevToolConfig>,
}

impl DevToolsStore {
    /// Adds a new [`DevTool`].
    ///
    /// If possible, prefer calling [`App::register_dev_tool`].
    pub fn add(&mut self, dev_tool: DevToolConfig) {
        self.dev_tools.insert(dev_tool.id.clone(), dev_tool);
    }

    /// Removes a [`DevTool`].
    pub fn remove(&mut self, id: &DevToolId) {
        self.dev_tools.remove(id);
    }

    /// Returns a reference to the given [`DevTool`] if present.
    pub fn get(&self, id: &DevToolId) -> Option<&DevToolConfig> {
        self.dev_tools.get(id)
    }

    /// Returns a mutable reference to the given [`DevTool`] if present.
    pub fn get_mut(&mut self, id: &DevToolId) -> Option<&mut DevToolConfig> {
        self.dev_tools.get_mut(id)
    }

    /// Returns an iterator over all [`DevTool`]s, by reference.
    pub fn iter(&self) -> impl Iterator<Item = &DevToolConfig> {
        self.dev_tools.values()
    }

    /// Returns an iterator over all [`DevTool`]s, by mutable reference.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut DevToolConfig> {
        self.dev_tools.values_mut()
    }
}

/// Extend [`App`] with new `register_dev_tool` function.
pub trait RegisterDevTool {
    /// Registers a new [`DevTool`].
    fn register_dev_tool(&mut self, dev_tool: DevToolConfig) -> &mut Self;
}

impl RegisterDevTool for App {
    fn register_dev_tool(&mut self, dev_tool: DevToolConfig) -> &mut Self {
        let mut dev_tools = self
            .world
            .get_resource_or_insert_with::<DevToolsStore>(Default::default);
        dev_tools.add(dev_tool);
        self
    }
}
