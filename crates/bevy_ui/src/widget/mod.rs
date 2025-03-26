//! This module contains the basic building blocks of Bevy's UI

mod button;
mod events;
mod image;
mod label;
mod text;

use crate::{
    AlignContent, AlignItems, AlignSelf, BorderRadius, Display, FlexDirection, FlexWrap,
    JustifyContent, JustifyItems, JustifySelf, Node, ReflectComponent, ReflectDefault, UiRect, Val,
};
use bevy_color::{Color, Srgba};
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
#[derive(Bundle, Default)]
pub struct WidgetBundle {
    /// Size and layout info
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

Node styling
- CSS layout parameters
- Theme/ palette
- Local style overrides

Interactions
- Focus
- Hover
- Click
- Touch (?)
- Drag
- DragInternal (?)
- Keyboard
*/

impl WidgetBundle {
    /// Set width and height to 100%
    #[inline]
    pub fn fill_parent(self) -> Self {
        Self {
            node: Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn width(self, w: impl Into<Val>) -> Self {
        Self {
            node: Node {
                width: w.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn height(self, h: impl Into<Val>) -> Self {
        Self {
            node: Node {
                height: h.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn display(self, d: impl Into<Display>) -> Self {
        Self {
            node: Node {
                display: d.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn flex_direction(self, d: impl Into<FlexDirection>) -> Self {
        Self {
            node: Node {
                flex_direction: d.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn flex_wrap(self, w: impl Into<FlexWrap>) -> Self {
        Self {
            node: Node {
                flex_wrap: w.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn align_items(self, a: impl Into<AlignItems>) -> Self {
        Self {
            node: Node {
                align_items: a.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn align_content(self, a: impl Into<AlignContent>) -> Self {
        Self {
            node: Node {
                align_content: a.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn align_self(self, a: impl Into<AlignSelf>) -> Self {
        Self {
            node: Node {
                align_self: a.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn justify_items(self, a: impl Into<JustifyItems>) -> Self {
        Self {
            node: Node {
                justify_items: a.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn justify_content(self, a: impl Into<JustifyContent>) -> Self {
        Self {
            node: Node {
                justify_content: a.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn justify_self(self, a: impl Into<JustifySelf>) -> Self {
        Self {
            node: Node {
                justify_self: a.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn padding(self, p: impl Into<UiRect>) -> Self {
        Self {
            node: Node {
                padding: p.into(),
                ..self.node
            },
            ..self
        }
    }
    #[inline]
    pub fn margin(self, m: impl Into<UiRect>) -> Self {
        Self {
            node: Node {
                margin: m.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn border(self, b: impl Into<UiRect>) -> Self {
        Self {
            node: Node {
                border: b.into(),
                ..self.node
            },
            ..self
        }
    }

    #[inline]
    pub fn border_radius(self, b: impl Into<BorderRadius>) -> Self {
        Self {
            border_radius: b.into(),
            ..self
        }
    }
}

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
