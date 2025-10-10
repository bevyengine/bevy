use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::component::Mutable;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityDoesNotExistError;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::query::QueryEntityError;
use bevy_ecs::query::With;
use bevy_ecs::system::Query;
use bevy_ecs::system::SystemParam;

use crate::TextColor;
use crate::TextFont;
use crate::TextSection;

/// Text writer error
pub struct TextWriterError;

#[derive(SystemParam)]
/// Text writer
pub struct TextWriter<'w, 's, T: Component<Mutability = Mutable>> {
    children_query: Query<'w, 's, &'static Children, With<TextSection>>,
    text_query: Query<'w, 's, &'static mut T>,
    color_query: Query<'w, 's, &'static mut TextColor>,
    font_query: Query<'w, 's, &'static mut TextFont>,
}

impl<'w, 's, T: Component<Mutability = Mutable> + From<String>> TextWriter<'w, 's, T> {
    /// set text
    pub fn set_text(
        &mut self,
        entity: Entity,
        index: usize,
        text: String,
    ) -> Result<(), TextWriterError> {
        if index == 0 {
            self.text_query
                .get_mut(entity)
                .map(|mut section| *section = T::from(text))
                .map_err(|_| TextWriterError)
        } else {
            self.children_query
                .iter_descendants_depth_first(entity)
                .nth(index - 1)
                .ok_or(TextWriterError)
                .and_then(|entity| self.text_query.get_mut(entity).map_err(|_| TextWriterError))
                .map(|mut section| *section = T::from(text))
        }
    }

    /// set color
    pub fn set_color(
        &mut self,
        entity: Entity,
        index: usize,
        color: impl Into<Color>,
    ) -> Result<(), TextWriterError> {
        if index == 0 {
            self.color_query
                .get_mut(entity)
                .map(|mut section| section.0 = color.into())
                .map_err(|_| TextWriterError)
        } else {
            self.children_query
                .iter_descendants_depth_first(entity)
                .nth(index - 1)
                .ok_or(TextWriterError)
                .and_then(|entity| {
                    self.color_query
                        .get_mut(entity)
                        .map_err(|_| TextWriterError)
                })
                .map(|mut section| section.0 = color.into())
        }
    }

    /// set font
    pub fn set_font(
        &mut self,
        entity: Entity,
        index: usize,
        font: TextFont,
    ) -> Result<(), TextWriterError> {
        if index == 0 {
            self.font_query
                .get_mut(entity)
                .map(|mut section| *section = font)
                .map_err(|_| TextWriterError)
        } else {
            self.children_query
                .iter_descendants_depth_first(entity)
                .nth(index - 1)
                .ok_or(TextWriterError)
                .and_then(|entity| self.font_query.get_mut(entity).map_err(|_| TextWriterError))
                .map(|mut section| *section = font)
        }
    }
}
