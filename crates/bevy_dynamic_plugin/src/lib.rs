#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Bevy's dynamic plugin loading functionality.
//!
//! This crate allows loading dynamic libraries (`.dylib`, `.so`) that export a single
//! [`Plugin`](bevy_app::Plugin). For usage, see [`dynamically_load_plugin`].
//!
//! # Deprecation
//!
//! The current dynamic plugin system is unsound and will be removed in 0.15. You may be interested
//! in the [Alternatives](#alternatives) listed below. If your use-case is not supported, please
//! consider commenting on [#13080](https://github.com/bevyengine/bevy/pull/13080) describing how
//! you use dynamic plugins in your project.
//!
//! # Warning
//!
//! Note that dynamic linking and loading is inherently unsafe because it allows executing foreign
//! code. Additionally, Rust does not have a stable ABI and may produce
//! incompatible libraries across Rust versions, or even subsequent compilations. This will not work
//! well in scenarios such as modding, but can work if the dynamic plugins and the main app are
//! built at the same time, such as with Downloadable Content (DLC) packs.
//!
//! # Alternatives
//!
//! You may be interested in these safer alternatives:
//!
//! - [Bevy Assets - Scripting]: Scripting and modding libraries for Bevy
//! - [Bevy Assets - Development tools]: Hot reloading and other development functionality
//! - [`stabby`]: Stable Rust ABI
//!
//! [Bevy Assets - Scripting]: https://bevyengine.org/assets/#scripting
//! [Bevy Assets - Development tools]: https://bevyengine.org/assets/#development-tools
//! [`stabby`]: https://github.com/ZettaScaleLabs/stabby

mod loader;

pub use loader::*;
