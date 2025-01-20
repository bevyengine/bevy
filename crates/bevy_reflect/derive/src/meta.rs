use bevy_macro_utils::BevyManifest;
use syn::Path;

/// Returns the correct path for `bevy_reflect`.
pub(crate) fn get_bevy_reflect_path() -> Path {
    let mut path = BevyManifest::shared().get_path("bevy_reflect");
    path.leading_colon = None;
    path
}
