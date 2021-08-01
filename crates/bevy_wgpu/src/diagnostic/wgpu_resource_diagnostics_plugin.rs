use crate::renderer::WgpuRenderResourceContext;
use bevy_app::prelude::*;
use bevy_diagnostic::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_ecs::system::{Res, ResMut};
use bevy_render::renderer::RenderResourceContext;

#[derive(Default)]
pub struct WgpuResourceDiagnosticsPlugin;

impl Plugin for WgpuResourceDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::diagnostic_system);
    }
}

impl WgpuResourceDiagnosticsPlugin {
    pub const BIND_GROUPS: DiagnosticId =
        DiagnosticId::from_u128(21302464753369276741568507794995836890);
    pub const BIND_GROUP_IDS: DiagnosticId =
        DiagnosticId::from_u128(283571569334075937453357861280307923122);
    pub const BIND_GROUP_LAYOUTS: DiagnosticId =
        DiagnosticId::from_u128(96406067032931216377076410852598331304);
    pub const BUFFERS: DiagnosticId =
        DiagnosticId::from_u128(133146619577893994787249934474491530491);
    pub const RENDER_PIPELINES: DiagnosticId =
        DiagnosticId::from_u128(278527620040377353875091478462209885377);
    pub const SAMPLERS: DiagnosticId =
        DiagnosticId::from_u128(305855369913076220671125671543184691267);
    pub const SHADER_MODULES: DiagnosticId =
        DiagnosticId::from_u128(287681470908132753275843248383768232237);
    pub const SWAP_CHAINS: DiagnosticId =
        DiagnosticId::from_u128(199253035828743332241465305105689014605);
    pub const SWAP_CHAIN_OUTPUTS: DiagnosticId =
        DiagnosticId::from_u128(112048874168736161226721327099863374234);
    pub const TEXTURES: DiagnosticId =
        DiagnosticId::from_u128(305955424195390184883220102469231911115);
    pub const TEXTURE_VIEWS: DiagnosticId =
        DiagnosticId::from_u128(257307432866562594739240898780307437578);
    pub const WINDOW_SURFACES: DiagnosticId =
        DiagnosticId::from_u128(108237028251680341878766034324149135605);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(
            Self::WINDOW_SURFACES,
            "window_surfaces",
            10,
        ));

        diagnostics.add(Diagnostic::new(Self::SWAP_CHAINS, "swap_chains", 10));

        diagnostics.add(Diagnostic::new(
            Self::SWAP_CHAIN_OUTPUTS,
            "swap_chain_outputs",
            10,
        ));

        diagnostics.add(Diagnostic::new(Self::BUFFERS, "buffers", 10));

        diagnostics.add(Diagnostic::new(Self::TEXTURES, "textures", 10));

        diagnostics.add(Diagnostic::new(Self::TEXTURE_VIEWS, "texture_views", 10));

        diagnostics.add(Diagnostic::new(Self::SAMPLERS, "samplers", 10));

        diagnostics.add(Diagnostic::new(Self::BIND_GROUP_IDS, "bind_group_ids", 10));
        diagnostics.add(Diagnostic::new(Self::BIND_GROUPS, "bind_groups", 10));

        diagnostics.add(Diagnostic::new(
            Self::BIND_GROUP_LAYOUTS,
            "bind_group_layouts",
            10,
        ));

        diagnostics.add(Diagnostic::new(Self::SHADER_MODULES, "shader_modules", 10));

        diagnostics.add(Diagnostic::new(
            Self::RENDER_PIPELINES,
            "render_pipelines",
            10,
        ));
    }

    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        render_resource_context: Res<Box<dyn RenderResourceContext>>,
    ) {
        let render_resource_context = render_resource_context
            .downcast_ref::<WgpuRenderResourceContext>()
            .unwrap();

        diagnostics.add_measurement(
            Self::WINDOW_SURFACES,
            render_resource_context
                .resources
                .window_surfaces
                .read()
                .len() as f64,
        );

        diagnostics.add_measurement(
            Self::SWAP_CHAINS,
            render_resource_context
                .resources
                .window_swap_chains
                .read()
                .len() as f64,
        );

        diagnostics.add_measurement(
            Self::SWAP_CHAIN_OUTPUTS,
            render_resource_context
                .resources
                .swap_chain_frames
                .read()
                .len() as f64,
        );

        diagnostics.add_measurement(
            Self::BUFFERS,
            render_resource_context.resources.buffers.read().len() as f64,
        );

        diagnostics.add_measurement(
            Self::TEXTURES,
            render_resource_context.resources.textures.read().len() as f64,
        );

        diagnostics.add_measurement(
            Self::TEXTURE_VIEWS,
            render_resource_context.resources.texture_views.read().len() as f64,
        );

        diagnostics.add_measurement(
            Self::SAMPLERS,
            render_resource_context.resources.samplers.read().len() as f64,
        );

        diagnostics.add_measurement(
            Self::BIND_GROUP_IDS,
            render_resource_context.resources.bind_groups.read().len() as f64,
        );

        let mut bind_group_count = 0;
        for bind_group in render_resource_context
            .resources
            .bind_groups
            .read()
            .values()
        {
            bind_group_count += bind_group.bind_groups.len();
        }

        diagnostics.add_measurement(Self::BIND_GROUPS, bind_group_count as f64);

        diagnostics.add_measurement(
            Self::BIND_GROUP_LAYOUTS,
            render_resource_context
                .resources
                .bind_group_layouts
                .read()
                .len() as f64,
        );

        diagnostics.add_measurement(
            Self::SHADER_MODULES,
            render_resource_context
                .resources
                .shader_modules
                .read()
                .len() as f64,
        );

        diagnostics.add_measurement(
            Self::RENDER_PIPELINES,
            render_resource_context
                .resources
                .render_pipelines
                .read()
                .len() as f64,
        );
    }
}
