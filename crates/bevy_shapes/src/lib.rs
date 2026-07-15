#![forbid(unsafe_code)]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(rustdoc_internals))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod primitives;

#[cfg(all(feature = "meshing", feature = "alloc"))]
pub mod meshing;

#[cfg(feature = "bounding")]
pub mod bounding;

#[cfg(feature = "sampling")]
pub mod sampling;

#[cfg(feature = "gizmos")]
pub mod gizmos;
