use bevy_macro_utils::BevyManifest;
use encase_derive_impl::{implement, syn};

const BEVY: &str = "bevy";
const BEVY_RENDER: &str = "bevy_render";
const ENCASE: &str = "encase";

fn bevy_encase_path() -> syn::Path {
    let bevy_manifest = BevyManifest::default();
    bevy_manifest
        .maybe_get_path(BEVY)
        .map(|bevy_path| {
            let mut segments = bevy_path.segments;
            segments.push(BevyManifest::parse_str("render"));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .or_else(|| bevy_manifest.maybe_get_path(BEVY_RENDER))
        .map(|bevy_render_path| {
            let mut segments = bevy_render_path.segments;
            segments.push(BevyManifest::parse_str("render_resource"));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .map(|path| {
            let mut segments = path.segments;
            segments.push(BevyManifest::parse_str(ENCASE));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .unwrap_or_else(|| bevy_manifest.get_path(ENCASE))
}

implement!(bevy_encase_path());
