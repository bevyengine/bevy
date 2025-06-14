use bevy_ecs::component::Component;
use cosmic_text::Buffer;
use cosmic_text::Editor;
use cosmic_text::Metrics;

#[derive(Component, Debug)]
pub struct TextInputBuffer {
    pub editor: Editor<'static>,
}

impl Default for TextInputBuffer {
    fn default() -> Self {
        Self {
            editor: Editor::new(Buffer::new_empty(Metrics::new(20.0, 20.0))),
        }
    }
}
