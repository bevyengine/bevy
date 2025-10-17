use bevy_ecs::resource::Resource;
use parley::swash::scale::ScaleContext;
use parley::FontContext;
use parley::LayoutContext;

#[derive(Resource, Default)]
pub struct TextContext {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext,
    pub scale_cx: ScaleContext,
}
