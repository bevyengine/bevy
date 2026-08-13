#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_cfg, rustdoc_internals))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]

//! This crate provides a way to extract component information from
//! an app’s main world into a sub world.
//!
//! The easiest way to set up extract is to add the [`ExtractPlugin`] for your [`AppLabel`](`bevy_app::AppLabel`).
//!
//! Then derive `ExtractComponent` or `ExtractResource` - ensure that you specify the `extract_app` attribute.
//!
//! ```ignore
//! #[derive(Component, Clone, Debug, ExtractComponent)]
//! #[extract_app(SomeApp)]
//! struct SomeComponent;
//! ```
//!
//! This adds the mechanism to first sync the entities from the main world to the sub world.
//! And then sync the component data from the main entity to the sub entity.
//!
//! More complex use cases may want to manually implement the `ExtractComponent` or `ExtractResource` traits directly.
//!
//! For higher performance needs use the [`ExtractInstance`](`crate::extract_instances::ExtractInstance`) trait.
//!
//! The sub app can access the main world in the [`ExtractSchedule`](`crate::ExtractSchedule`).
//! Adding a system with a query wrapped in [`Extract`](`crate::Extract`) and it will run against the main app world.
//!
//! [`ExtractComponent`]: crate::extract_component::ExtractComponent
//! [`ExtractResource`]: crate::extract_resource::ExtractResource

extern crate alloc;

pub mod extract_component;
pub mod extract_instances;
pub mod extract_param;
pub mod extract_plugin;
pub mod extract_resource;
pub mod sync_component;
pub mod sync_world;

pub use extract_param::Extract;
pub use extract_plugin::*;
pub use extract_plugin::{ExtractSchedule, MainWorld};
pub use sync_world::*;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_extract;
