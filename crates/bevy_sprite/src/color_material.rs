use bevy_app::{EventReader, Events, ManualEventReader};
use bevy_asset::{self, AssetEvent, Assets, Handle};
use bevy_ecs::system::{Local, Res, ResMut};
use bevy_reflect::TypeUuid;
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};
use bevy_utils::{HashMap, HashSet};

#[derive(Debug, RenderResources, ShaderDefs, TypeUuid)]
#[uuid = "506cff92-a9f3-4543-862d-6851c7fdfc99"]
pub struct ColorMaterial {
    pub color: Color,
    #[shader_def]
    pub texture: Option<Handle<Texture>>,
}

impl ColorMaterial {
    pub fn color(color: Color) -> Self {
        ColorMaterial {
            color,
            texture: None,
        }
    }

    pub fn texture(texture: Handle<Texture>) -> Self {
        ColorMaterial {
            color: Color::WHITE,
            texture: Some(texture),
        }
    }

    pub fn modulated_texture(texture: Handle<Texture>, color: Color) -> Self {
        ColorMaterial {
            color,
            texture: Some(texture),
        }
    }
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::rgb(1.0, 1.0, 1.0),
            texture: None,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> Self {
        ColorMaterial::color(color)
    }
}

impl From<Handle<Texture>> for ColorMaterial {
    fn from(texture: Handle<Texture>) -> Self {
        ColorMaterial::texture(texture)
    }
}

// Temporary solution for sub-assets change handling, see https://github.com/bevyengine/bevy/issues/1161#issuecomment-780467768
// TODO: should be removed when pipelined rendering is done
#[allow(clippy::type_complexity)]
pub(crate) fn material_texture_detection_system(
    mut texture_to_material: Local<HashMap<Handle<Texture>, HashSet<Handle<ColorMaterial>>>>,
    mut material_to_texture: Local<HashMap<Handle<ColorMaterial>, Handle<Texture>>>,
    materials: Res<Assets<ColorMaterial>>,
    mut texture_events: EventReader<AssetEvent<Texture>>,
    (mut material_events_reader, mut material_events): (
        Local<ManualEventReader<AssetEvent<ColorMaterial>>>,
        ResMut<Events<AssetEvent<ColorMaterial>>>,
    ),
) {
    for event in material_events_reader.iter(&material_events) {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(texture) = materials.get(handle).and_then(|mat| mat.texture.as_ref()) {
                    material_to_texture.insert(handle.clone_weak(), texture.clone_weak());
                    texture_to_material
                        .entry(texture.clone_weak())
                        .or_default()
                        .insert(handle.clone_weak());
                }
            }
            AssetEvent::Modified { handle } => {
                let old_texture = material_to_texture.get(handle).cloned();
                match (
                    materials.get(handle).and_then(|mat| mat.texture.as_ref()),
                    old_texture,
                ) {
                    (None, None) => (),
                    (Some(texture), None) => {
                        material_to_texture.insert(handle.clone_weak(), texture.clone_weak());
                        texture_to_material
                            .entry(texture.clone_weak())
                            .or_default()
                            .insert(handle.clone_weak());
                    }
                    (None, Some(texture)) => {
                        material_to_texture.remove(handle);
                        texture_to_material
                            .entry(texture.clone_weak())
                            .or_default()
                            .remove(handle);
                    }
                    (Some(new_texture), Some(old_texture)) => {
                        if &old_texture == new_texture {
                            continue;
                        }
                        material_to_texture.insert(handle.clone_weak(), new_texture.clone_weak());
                        texture_to_material
                            .entry(new_texture.clone_weak())
                            .or_default()
                            .insert(handle.clone_weak());
                        texture_to_material
                            .entry(old_texture.clone_weak())
                            .or_default()
                            .remove(handle);
                    }
                }
            }
            AssetEvent::Removed { handle } => {
                if let Some(texture) = materials.get(handle).and_then(|mat| mat.texture.as_ref()) {
                    material_to_texture.remove(handle);
                    texture_to_material
                        .entry(texture.clone_weak())
                        .or_default()
                        .remove(handle);
                }
            }
        }
    }

    let mut changed_textures = HashSet::default();
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle }
            | AssetEvent::Modified { handle }
            | AssetEvent::Removed { handle } => {
                changed_textures.insert(handle);
            }
        }
    }

    for texture_handle in changed_textures.iter() {
        if let Some(materials) = texture_to_material.get(texture_handle) {
            for material in materials.iter() {
                material_events.send(AssetEvent::Modified {
                    handle: material.clone_weak(),
                });
            }
        }
    }
}
