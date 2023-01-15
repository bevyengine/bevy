use crate::PBR_OVERRIDE_HANDLE;
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::prelude::{DetectChanges, EventReader, Res, ResMut, Resource};
use bevy_render::render_resource::{Shader, ShaderImport};
use bevy_utils::tracing::warn;
use naga_oil::compose::ImportDefinition;

#[derive(Default, Resource)]
pub struct PbrShaderFunctionOverrides {
    pub overrides: Vec<Handle<Shader>>,
}

pub(crate) fn update_shader_overrides(
    override_list: Res<PbrShaderFunctionOverrides>,
    mut load_events: EventReader<AssetEvent<Shader>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let run_condition = override_list.is_changed()
        || load_events
            .iter()
            .map(|ev| match ev {
                AssetEvent::Created { handle }
                | AssetEvent::Modified { handle }
                | AssetEvent::Removed { handle } => handle,
            })
            .any(|handle| override_list.overrides.contains(handle));

    load_events.clear();

    if run_condition {
        // collect user imports
        let imports: Vec<_> = override_list
            .overrides
            .iter()
            // skip any handles that are not (yet) loaded
            .flat_map(|handle| shaders.get(handle).map(|shader| (handle, shader)))
            .flat_map(
                |(handle, shader)| match (&shader.import_path, &shader.path) {
                    // prefer import_path
                    (Some(import_path), _) => Some(import_path.clone()),
                    // fall back to "path"
                    (None, Some(path)) => Some(ShaderImport::AssetPath(path.clone())),
                    // skip any shaders that don't have an import path or a path
                    _ => {
                        warn!(
                            "pbr shader function with handle {:?} has no `import_path` or `path`",
                            handle
                        );
                        None
                    }
                },
            )
            .collect();

        let mut shader = shaders.get_mut(&PBR_OVERRIDE_HANDLE.typed_weak()).unwrap();
        shader.additional_imports = imports
            .iter()
            .map(|import| ImportDefinition {
                import: import.as_str().to_owned(),
                ..Default::default()
            })
            .collect();
        shader.imports = imports;
    }
}
