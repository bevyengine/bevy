use crate::PBR_OVERRIDE_HANDLE;
use bevy_asset::{Assets, Handle};
use bevy_ecs::system::{Res, ResMut};
use bevy_render::render_resource::{Shader, Source};
use bevy_utils::tracing::warn;

#[derive(Default)]
pub struct PbrShaderFunctionOverrides {
    pub overrides: Vec<Handle<Shader>>,
}

pub(crate) fn update_shader_overrides(
    override_list: Res<PbrShaderFunctionOverrides>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    if override_list.is_changed() {
        // collect user module declarations
        let override_declarations = override_list.overrides.iter().flat_map(|overrider| {
            if let Some(Shader {
                import_path: Some(path),
                ..
            }) = shaders.get(overrider)
            {
                Some(format!("#import {}\n", path.as_str()))
            } else {
                warn!(
                    "pbr shader function override with handle {:?} has no import path",
                    overrider
                );
                None
            }
        });

        // and import paths
        let imports = override_list
            .overrides
            .iter()
            .flat_map(|overrider| {
                if let Some(Shader {
                    import_path: Some(path),
                    ..
                }) = shaders.get(overrider)
                {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        // build and store final shader
        let override_shader_string = format!(
            "#define_import_path bevy_pbr::user_overrides\n{}\n",
            override_declarations.collect::<Vec<_>>().join("\n")
        );
        let mut shader = shaders.get_mut(&PBR_OVERRIDE_HANDLE.typed_weak()).unwrap();
        shader.source = Source::Wgsl(override_shader_string.into());
        shader.imports = imports;
    }
}
