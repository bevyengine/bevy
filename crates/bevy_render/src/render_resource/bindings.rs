use crate::{
    auto_binding::{
        AddAutoBinding, AutoBindGroup, AutoBindGroupLayoutEntry, AutoBinding, ShaderBindingName,
    },
    globals::{GlobalsBuffer, GlobalsUniform},
    render_resource::*,
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_app::App;
use bevy_asset::{load_internal_asset_with_path, HandleUntyped};
use bevy_ecs::{
    prelude::Entity,
    system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
};
use bevy_log::debug;
use bevy_reflect::TypeUuid;

pub const CORE_VIEW_TYPES_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12391644269375481087);

pub const CORE_VIEW_BINDINGS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 655617546865762511);

pub fn setup_core_view_bindings(app: &mut App) {
    load_internal_asset_with_path!(
        app,
        CORE_VIEW_TYPES_HANDLE,
        "core_types.wgsl",
        Shader::from_wgsl_with_path
    );

    load_internal_asset_with_path!(
        app,
        CORE_VIEW_BINDINGS_HANDLE,
        "core_bindings.wgsl",
        Shader::from_wgsl_with_path
    );
}

pub trait AddCoreBindings {
    fn add_core_view_bindings<G: AutoBindGroup>(&mut self) -> &mut Self;
}

impl AddCoreBindings for App {
    fn add_core_view_bindings<G: AutoBindGroup>(&mut self) -> &mut Self {
        self.add_auto_binding::<G, ViewUniformBinding>()
            .add_auto_binding::<G, GlobalsUniformBinding>()
    }
}

pub struct ViewUniformBinding;
impl AutoBinding for ViewUniformBinding {
    type LayoutParam = ();
    type BindingParam = (SRes<ViewUniforms>, SQuery<Read<ViewUniformOffset>>);

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_render::core_bindings", "view")
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(ViewUniform::min_size()),
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        (view_uniforms, offsets): &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        let Ok(dynamic_offset) = offsets.get(entity) else {
            // warn!("no view offset for {:?}", entity);
            return None;
        };

        debug!("{:?} view dyn offset: {}", entity, dynamic_offset.offset);

        view_uniforms.uniforms.owned_binding(dynamic_offset.offset)
    }
}

pub struct GlobalsUniformBinding;
impl AutoBinding for GlobalsUniformBinding {
    type LayoutParam = ();
    type BindingParam = SRes<GlobalsBuffer>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_render::core_bindings", "globals")
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(GlobalsUniform::min_size()),
            },
            count: None,
        }
    }
    fn binding_source(
        _entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param.buffer.owned_binding()
    }
}
