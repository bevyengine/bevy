#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

extern crate alloc;

pub mod wireframe;

mod parallax;
mod pbr_material;

pub use parallax::*;
pub use pbr_material::*;

/// The PBR prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{parallax::ParallaxMappingMethod, pbr_material::StandardMaterial};
}

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, AssetApp, Assets, Handle};
use bevy_color::Color;
use bevy_render::{render_resource::Shader, RenderDebugFlags};
use bevy_render_3d::{decal::ForwardDecalPlugin, MaterialPlugin};

const PBR_TYPES_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("b0330585-2335-4268-9032-a6c4c2d932f6");
const PBR_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("13834c18-c7ec-4c4b-bbbd-432c3ba4cace");
const PBR_FRAGMENT_HANDLE: Handle<Shader> = weak_handle!("1bd3c10d-851b-400c-934a-db489d99cc50");
const PBR_SHADER_HANDLE: Handle<Shader> = weak_handle!("0eba65ed-3e5b-4752-93ed-e8097e7b0c84");
const PBR_PREPASS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("9afeaeab-7c45-43ce-b322-4b97799eaeb9");
const PBR_FUNCTIONS_HANDLE: Handle<Shader> = weak_handle!("815b8618-f557-4a96-91a5-a2fb7e249fb0");
const PBR_AMBIENT_HANDLE: Handle<Shader> = weak_handle!("4a90b95b-112a-4a10-9145-7590d6f14260");
const PARALLAX_MAPPING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("6cf57d9f-222a-429a-bba4-55ba9586e1d4");
const PBR_PREPASS_FUNCTIONS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("77b1bd3a-877c-4b2c-981b-b9c68d1b774a");
const RGB9E5_FUNCTIONS_HANDLE: Handle<Shader> =
    weak_handle!("90c19aa3-6a11-4252-8586-d9299352e94f");

/// Sets up the entire PBR infrastructure of bevy.
pub struct PbrPlugin {
    /// Controls if the prepass is enabled for the [`StandardMaterial`].
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    pub prepass_enabled: bool,
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl Default for PbrPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            debug_flags: RenderDebugFlags::default(),
        }
    }
}

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PBR_TYPES_SHADER_HANDLE,
            "render/pbr_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_BINDINGS_SHADER_HANDLE,
            "render/pbr_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_FUNCTIONS_HANDLE,
            "render/pbr_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            RGB9E5_FUNCTIONS_HANDLE,
            "render/rgb9e5.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_AMBIENT_HANDLE,
            "render/pbr_ambient.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_FRAGMENT_HANDLE,
            "render/pbr_fragment.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, PBR_SHADER_HANDLE, "render/pbr.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            PBR_PREPASS_FUNCTIONS_SHADER_HANDLE,
            "render/pbr_prepass_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_PREPASS_SHADER_HANDLE,
            "render/pbr_prepass.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PARALLAX_MAPPING_SHADER_HANDLE,
            "render/parallax_mapping.wgsl",
            Shader::from_wgsl
        );

        app.register_asset_reflect::<StandardMaterial>()
            .add_plugins((
                MaterialPlugin::<StandardMaterial> {
                    prepass_enabled: self.prepass_enabled,
                    debug_flags: self.debug_flags,
                    ..Default::default()
                },
                ForwardDecalPlugin::<StandardMaterial>::default(),
            ));

        // Initialize the default material handle.
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                &Handle::<StandardMaterial>::default(),
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.5),
                    ..Default::default()
                },
            );
    }
}
