use bevy_ecs::resource::Resource;
use parley::FontContext;

#[derive(Resource)]
pub struct TextContext {
    pub font_cx: FontContext,
}
