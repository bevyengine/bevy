//! basic bevy_2d text input
use bevy::{
    color::palettes::css::*,
    math::ops,
    prelude::*,
    sprite::Anchor,
    text::{
        FontSmoothing, LineBreak, Placeholder, TextBounds, TextInputBuffer, TextInputTarget,
        TextLayoutInfo, UndoHistory,
    },
};
use bevy_render::Extract;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_systems(PostUpdate, update_targets)
        .add_systems(ExtractSchedule, extract_text_input)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Transform::default(),
        TextInputBuffer::default(),
        UndoHistory::default(),
        Placeholder::new("type here.."),
    ));
}

fn update_targets() {}

fn update() {}

fn extract_text_input(
    query: Extract<Query<(Entity, &GlobalTransform, &TextLayoutInfo, &TextInputTarget)>>,
) {
}
