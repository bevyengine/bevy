use bevy_app::Plugin;
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::{
    component::Component,
    lifecycle::Add,
    observer::On,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Query, Res},
    world::FromWorld,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_render::render_resource::AsBindGroup;
use bevy_shader::ShaderRef;
use bevy_ui_render::ui_material::{MaterialNode, UiMaterial};

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
pub(crate) struct AlphaPatternMaterial {}

impl UiMaterial for AlphaPatternMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/alpha_pattern.wgsl".into()
    }
}

#[derive(Resource)]
pub(crate) struct AlphaPatternResource(pub(crate) Handle<AlphaPatternMaterial>);

impl FromWorld for AlphaPatternResource {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        let mut ui_materials = world
            .get_resource_mut::<Assets<AlphaPatternMaterial>>()
            .unwrap();
        Self(ui_materials.add(AlphaPatternMaterial::default()))
    }
}

/// Marker that tells us we want to fill in the [`MaterialNode`] with the alpha material.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct AlphaPattern;

/// Observer to fill in the material handle (since we don't have access to the materials asset
/// in the template)
fn on_add_alpha_pattern(
    ev: On<Add, AlphaPattern>,
    mut q_material_node: Query<&mut MaterialNode<AlphaPatternMaterial>>,
    r_material: Res<AlphaPatternResource>,
) {
    if let Ok(mut material) = q_material_node.get_mut(ev.target()) {
        material.0 = r_material.0.clone();
    }
}

/// Plugin which registers the systems for updating the button styles.
pub struct AlphaPatternPlugin;

impl Plugin for AlphaPatternPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_observer(on_add_alpha_pattern);
    }
}
