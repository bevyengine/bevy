mod glsl;
mod layout;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream as CompilerTokenStream;

use syn::{parse_macro_input, DeriveInput, Path};

#[proc_macro_derive(AsStd140)]
pub fn derive_as_std140(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = layout::emit(input, "Std140", "std140", 16);

    CompilerTokenStream::from(expanded)
}

#[proc_macro_derive(AsStd430)]
pub fn derive_as_std430(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = layout::emit(input, "Std430", "std430", 0);

    CompilerTokenStream::from(expanded)
}

#[proc_macro_derive(GlslStruct)]
pub fn derive_glsl_struct(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = glsl::emit(input);

    CompilerTokenStream::from(expanded)
}

const BEVY: &str = "bevy";
const BEVY_CREVICE: &str = "bevy_crevice";
const BEVY_RENDER: &str = "bevy_render";

fn bevy_crevice_path() -> Path {
    let bevy_manifest = BevyManifest::default();
    bevy_manifest
        .maybe_get_path(crate::BEVY)
        .map(|bevy_path| {
            let mut segments = bevy_path.segments;
            segments.push(BevyManifest::parse_str("render"));
            Path {
                leading_colon: None,
                segments,
            }
        })
        .or_else(|| bevy_manifest.maybe_get_path(crate::BEVY_RENDER))
        .map(|bevy_render_path| {
            let mut segments = bevy_render_path.segments;
            segments.push(BevyManifest::parse_str("render_resource"));
            Path {
                leading_colon: None,
                segments,
            }
        })
        .unwrap_or_else(|| bevy_manifest.get_path(crate::BEVY_CREVICE))
}
