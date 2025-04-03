#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub mod atmosphere;
pub mod volumetric_fog;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::atmosphere::{Atmosphere, AtmosphereSettings};
    #[doc(hidden)]
    pub use crate::volumetric_fog::{FogVolume, VolumetricFog};
}
