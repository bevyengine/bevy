use bevy_ecs::define_label;
use bevy_ecs::intern::Interned;
pub use bevy_material_macros::ShaderLabel;

define_label!(
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(ShaderLabel)]`"
    )]
    /// Labels used to uniquely identify types of material shaders
    ShaderLabel,
    SHADER_LABEL_INTERNER
);

/// A shorthand for `Interned<dyn RenderSubGraph>`.
pub type InternedShaderLabel = Interned<dyn ShaderLabel>;

pub use bevy_material_macros::DrawFunctionLabel;

define_label!(
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(DrawFunctionLabel)]`"
    )]
    /// Labels used to uniquely identify types of material shaders
    DrawFunctionLabel,
    DRAW_FUNCTION_LABEL_INTERNER
);

pub type InternedDrawFunctionLabel = Interned<dyn DrawFunctionLabel>;

// TODO: make this generic?
/// An identifier for a [`Draw`] function stored in [`DrawFunctions`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DrawFunctionId(pub u32);
