//! Common types used for text picking.

use bevy_app::App;
use bevy_ecs::{
    entity::{Entity, EntityBorrow},
    event::Event,
    hierarchy::ChildOf,
    query::QueryData,
    traversal::Traversal,
};
use bevy_picking::{
    backend::HitData,
    events::{
        Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Move,
        Out, Over, Pointer, Pressed, Released,
    },
};
use bevy_reflect::Reflect;
use bevy_render::camera::NormalizedRenderTarget;
use bevy_window::Window;
use cosmic_text::Cursor;

use crate::PositionedGlyph;

pub(crate) fn plugin(app: &mut App) {
    app.add_event::<TextPointer<Cancel>>()
        .add_event::<TextPointer<Click>>()
        .add_event::<TextPointer<Pressed>>()
        .add_event::<TextPointer<DragDrop>>()
        .add_event::<TextPointer<DragEnd>>()
        .add_event::<TextPointer<DragEnter>>()
        .add_event::<TextPointer<Drag>>()
        .add_event::<TextPointer<DragLeave>>()
        .add_event::<TextPointer<DragOver>>()
        .add_event::<TextPointer<DragStart>>()
        .add_event::<TextPointer<Move>>()
        .add_event::<TextPointer<Out>>()
        .add_event::<TextPointer<Over>>()
        .add_event::<TextPointer<Released>>();
}

/// Text-specific pointer event.
#[derive(Debug, Clone)]
pub struct TextPointer<E: Clone + Reflect + core::fmt::Debug> {
    /// The picked location in text.
    pub cursor: Cursor,
    /// The `PositionedGlyph` the the picked location in text.
    pub glyph: PositionedGlyph,
    /// The original `Pointer` event that triggered the `TextPointer` event.
    pub event: Pointer<E>,
}

impl<E> Event for TextPointer<E>
where
    E: Clone + Reflect + core::fmt::Debug,
{
    const AUTO_PROPAGATE: bool = true;
    type Traversal = TextPointerTraversal;
}

/// A traversal query (eg it implements [`Traversal`]) intended for use with [`TextPointer`] events.
///
/// This will always traverse to the parent, if the entity being visited has one. Otherwise, it
/// propagates to the pointer's window and stops there.
#[derive(QueryData)]
pub struct TextPointerTraversal {
    parent: Option<&'static ChildOf>,
    window: Option<&'static Window>,
}

impl<E> Traversal<TextPointer<E>> for TextPointerTraversal
where
    E: core::fmt::Debug + Clone + Reflect,
{
    fn traverse(item: Self::Item<'_>, pointer: &TextPointer<E>) -> Option<Entity> {
        let TextPointerTraversalItem { parent, window } = item;

        // Send event to parent, if it has one.
        if let Some(parent) = parent {
            return Some(parent.get());
        };

        // Otherwise, send it to the window entity (unless this is a window entity).
        if window.is_none() {
            if let NormalizedRenderTarget::Window(window_ref) =
                pointer.event.pointer_location.target
            {
                return Some(window_ref.entity());
            }
        }

        None
    }
}

/// Pointer event shared trait where `HitData` exists.
pub trait HasHit: Clone + Reflect + core::fmt::Debug {
    /// Provides access to the event's `HitData`.
    fn hit(&self) -> &HitData;
}

impl HasHit for Cancel {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Click {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Pressed {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for DragDrop {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for DragEnter {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for DragLeave {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for DragOver {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for DragStart {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Move {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Out {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Over {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
impl HasHit for Released {
    fn hit(&self) -> &HitData {
        &self.hit
    }
}
