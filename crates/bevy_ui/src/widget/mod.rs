//! This module contains the basic building blocks of Bevy's UI

mod builder;
mod button;
mod events;
mod image;
mod label;
mod text;

use crate::{
    AlignContent, AlignItems, AlignSelf, BackgroundColor, BorderColor, BorderRadius, Display,
    FlexDirection, FlexWrap, JustifyContent, JustifyItems, JustifySelf, Node, ReflectComponent,
    ReflectDefault, UiRect, Val,
};
use bevy_color::{Color, Srgba};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::With,
    system::{Commands, Query},
};
use bevy_math::Vec2;
use bevy_picking::events::{Over, Pointer};
use bevy_reflect::Reflect;

use bevy_text::TextColor;
pub use button::*;
pub use events::*;
pub use image::*;
pub use label::*;
pub use text::*;

pub enum Style {
    Color(ColorStyle),
    Border(BorderStyle),
}

/// A full color palette in use across
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ColorStyle {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub failure: Color,
    pub note: Color,
}

impl Default for ColorStyle {
    fn default() -> Self {
        // Solarized Light, stolen from iced-rs :p
        Self {
            background: Srgba::rgb_u8(0x00, 0x2b, 0x36).into(), // base03
            text: Srgba::rgb_u8(0x65, 0x7b, 0x83).into(),       // base00
            primary: Srgba::rgb_u8(0x2a, 0xa1, 0x98).into(),    // cyan
            secondary: Srgba::rgb_u8(0x2a, 0xa1, 0x98).into(),  // cyan
            success: Srgba::rgb_u8(0x85, 0x99, 0x00).into(),    // green
            failure: Srgba::rgb_u8(0xb5, 0x89, 0x00).into(),    // yellow
            note: Srgba::rgb_u8(0xdc, 0x32, 0x2f).into(),       // red
        }
    }
}

#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct BorderStyle {
    pub color: Color,
    pub size: Vec2,
    pub geom: BorderGeometry,
}

#[derive(Component, Default)]
pub struct ShadowStyle {}

#[derive(Default, Copy, Clone, Debug, Reflect)]
#[reflect(Default, Debug, Clone)]
pub enum BorderGeometry {
    Dotted,
    Dashed,
    Solid,
    Double,
    Groove,
    Ridge,
    Outset,
    #[default]
    None,
    Hidden,
}

/// A stack of style changes that can be pushed into or "rolled back" to previous ones
#[derive(Component, Default, Reflect)]
pub struct Rollback<T>(Vec<T>);

impl<T> Rollback<T> {
    pub fn push(&mut self, t: T) {
        self.0.push(t);
    }

    pub fn roll_back(&mut self) -> Option<T> {
        self.0.pop()
    }
}

/// All components of a basic widget entity
///
/// This type also implements two sets of "builder" APIs:
///
/// - Mapping `Node` attributes to specify basic layout information
///
/// - "Picking" interface with `.observe(...)`, which can be enabled or disabled dynamically via the
/// `UiActive` component
///
/// - Widget "event reactor" API that lets users specify their own UI event type which can be
/// emitted and handled by widgets for widget-to-widget communication.  See [Elm] for the underlying
/// model of constructing UI trees.
#[derive(Bundle, Default, Deref, DerefMut)]
pub struct WidgetBundle {
    /// Size and layout info
    #[deref]
    pub node: Node,
    /// Color scheme used in this widget tree
    pub color: ColorStyle,
    /// Border layout and color info
    pub border: BorderStyle,
    /// Rounded corners
    pub border_radius: BorderRadius,
    /// React to basic picking events
    pub picking: EventsReactor<PickingEvent>,
}

/*
 *** TODO

- Apply `CodeStyle`, `BorderStyle`, ... to relevant render components.  [DESIGN NEEDED]: new widgets inherit the styles of their parents.  An override in one widget would apply it consistently for ALL child widgets in its tree.

- Switch `WidgetBundle` API to a trait, which implements `Deref<Target=Node>` to make the ~90% of functions just map to the inner Node, then implement the ~10% of widget-specific functions on the type itself.

- Have defaults for `button()`, `checkbox()`, ... which return their respective `<Widget>Bundle` but let users override everything via their spective builder APIs.  Each specific bundle should `Deref` to its inner composing bundle.
*/

pub fn apply_observe(mut commands: Commands, q: Query<Entity, With<EventsReactor<PickingEvent>>>) {
    // fixme: ideally we don't want to re-apply `observe` on nodes that already do so, or where the
    // observation strategy hasn't changed.  Considering there's currently no way for a user to
    // insert their own effects here, this is probably a bad design.  Someone in code review will
    // complain about it :p
    for ent in q.iter() {
        commands.entity(ent).observe(
            |trig: Trigger<Pointer<Over>>, mut reactor: Query<&mut EventsReactor<PickingEvent>>| {
                let mut reactor = reactor.get_mut(trig.target()).unwrap();
                reactor
                    .0
                    .push_back(PickingEvent::Over(trig.event().clone()));
            },
        );
    }
}

pub fn apply_colorstyle(
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &ColorStyle,
        Option<&mut BackgroundColor>,
        Option<&mut TextColor>,
        Option<&mut BorderColor>,
    )>,
) {
    for (ent, color, opt_bg, opt_txt, opt_bord) in q.iter_mut() {
        match opt_bg {
            Some(mut bg_color) => *bg_color = BackgroundColor(color.background),
            _ => {
                commands
                    .entity(ent)
                    .insert(BackgroundColor(color.background));
            }
        };
    }
}
