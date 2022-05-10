use bevy_macro_utils::BevyManifest;
use syn::Path;

pub fn get_bevy_reflect_path() -> Path {
    BevyManifest::default().get_path("bevy_reflect")
}
