//! A loading bar widget.
//! Can be used for loading bars, but also health-bars, mana, those kind of things.

use bevy_ecs::{
    prelude::{Bundle, Component},
    query::With,
    system::Query,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_render::view::{ComputedVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::{BackgroundColor, FocusPolicy, Node, Size, Style, Val, ZIndex};

#[derive(Component, Default, Clone, Debug)]
pub struct LoadingBarWidget {
    progress: f32,
}

/// Marker component for the inner box of the loading bar.
#[derive(Component, Default, Clone, Debug)]
pub struct LoadingBarInner;

impl LoadingBarWidget {
    /// Creates a new [``LoadingBarWidget`].
    pub const fn new(progress: f32) -> Self {
        LoadingBarWidget { progress }
    }
    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    pub fn set_progress(&mut self, progress: f32) {
        if progress >= 0. && progress <= 1. {
            self.progress = progress;
        } else {
            warn!("Trying to set progress out of range");
        }
    }
}

/// A UI node that is an image
#[derive(Bundle, Clone, Debug, Default)]
pub struct LoadingBarWidgetBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,

    pub loading_bar: LoadingBarWidget,
    /// The background color, which serves as a "fill" for this node
    ///
    /// When combined with `UiImage`, tints the provided image.
    pub background_color: BackgroundColor,
    // /// The image of the node
    // pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

pub(crate) fn update_loading_bars(
    q: Query<(&LoadingBarWidget, &Children)>,
    mut inner: Query<&mut Style, With<LoadingBarInner>>,
) {
    for (widget, children) in q.iter() {
        for child in children.iter() {
            if let Ok(mut style) = inner.get_mut(*child) {
                style.size = Size::new(
                    Val::Percent(widget.get_progress() * 100.0),
                    Val::Percent(100.0),
                );
            }
        }
    }
}
