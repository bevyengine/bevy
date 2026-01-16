#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;

mod shader;
mod shader_cache;
pub use shader::*;
pub use shader_cache::*;

/// The shader prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::Shader;
}

#[doc(hidden)]
pub mod _macro {
    pub use bevy_asset;
}

/// Inline shader as an `embedded_asset` and load it permanently.
///
/// This works around a limitation of the shader loader not properly loading
/// dependencies of shaders.
#[macro_export]
macro_rules! load_shader_library {
    ($asset_server_provider: expr, $path: literal $(, $settings: expr)?) => {
        $crate::_macro::bevy_asset::embedded_asset!($asset_server_provider, $path);
        let handle: $crate::_macro::bevy_asset::prelude::Handle<$crate::prelude::Shader> =
            $crate::_macro::bevy_asset::load_embedded_asset!(
                $asset_server_provider,
                $path
                $(,$settings)?
            );
        core::mem::forget(handle);
    }
}
