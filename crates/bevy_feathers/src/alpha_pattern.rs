use bevy_asset::{Asset, Assets};
use bevy_ecs::{component::Component, lifecycle::HookContext, world::DeferredWorld};
use bevy_reflect::TypePath;
use bevy_render::render_resource::{AsBindGroup, ShaderRef};
use bevy_ui_render::ui_material::{MaterialNode, UiMaterial};

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
pub(crate) struct AlphaPatternMaterial {}

impl UiMaterial for AlphaPatternMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/alpha_pattern.wgsl".into()
    }
}

/// Marker that tells us we want to fill in the [`MaterialNode`] with the alpha material.
#[derive(Component, Default, Clone)]
#[require(MaterialNode<AlphaPatternMaterial>)]
#[component(on_add = on_add_alpha_pattern)]
pub(crate) struct AlphaPattern;

/// Observer to fill in the material handle (since we don't have access to the materials asset
/// in the template)
fn on_add_alpha_pattern(mut world: DeferredWorld, context: HookContext) {
    let mut materials = world.resource_mut::<Assets<AlphaPatternMaterial>>();

    let handle = if materials.is_empty() {
        materials.add(AlphaPatternMaterial::default())
    } else {
        let id = materials.iter().next().unwrap().0;
        materials.get_strong_handle(id).unwrap()
    };

    if let Some(mut material) = world
        .entity_mut(context.entity)
        .get_mut::<MaterialNode<AlphaPatternMaterial>>()
    {
        material.0 = handle;
    }
}
