#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Bevy's (deprecated) dynamic plugin loading functionality.
//! 
//! This crate used to allow loading plugins from dynamic libraries, but the implementation was
//! unsound, so it was deprecated in 0.14 and removed in 0.15. You may be interested in the
//! [Alternatives](#alternatives) listed below.
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
