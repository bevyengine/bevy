//! Pointer-driven focus: focus or blur entities in response to mouse/pointer clicks.
//!
//! This module is intentionally independent of any focus-navigation scheme (such as
//! [`tab_navigation`](crate::tab_navigation) or
//! [`directional_navigation`](crate::directional_navigation)). It only *requests* a focus change
//! by triggering a bubbling [`AcquireFocus`] event on the clicked entity;
//! how that request is resolved depends on which focus observers are installed.
//!
//! Add [`PointerFocusPlugin`] to enable it. Requires the `bevy_picking` feature.

use bevy_app::{App, Plugin};
use bevy_ecs::{
    entity::Entity,
    observer::On,
    query::With,
    system::{Commands, Query, ResMut},
};
use bevy_picking::events::PointerPress;
use bevy_window::PrimaryWindow;

use crate::{AcquireFocus, InputFocusVisible};

/// Observer which requests focus for a clicked entity.
///
/// On a pointer press this hides the focus indicator ([`InputFocusVisible`]) and triggers a
/// bubbling [`AcquireFocus`] on the clicked entity. It does not itself decide *what* becomes
/// focused — that is the job of whatever [`AcquireFocus`] resolver observers are installed (see
/// [`PointerFocusPlugin`]).
fn click_to_focus(
    press: On<PointerPress>,
    mut focus_visible: ResMut<InputFocusVisible>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    // Because `Pointer` is a bubbling event, we don't want to trigger an `AcquireFocus` event
    // for every ancestor, but only for the original entity. Also, users may want to stop
    // propagation on the pointer event at some point along the bubbling chain, so we need our
    // own dedicated event whose propagation we can control.
    if press.entity == press.original_event_target() {
        // Clicking hides focus
        if focus_visible.0 {
            focus_visible.0 = false;
        }
        // Search for a focusable parent entity, defaulting to window if none.
        if let Ok(window) = windows.single() {
            commands.trigger(AcquireFocus {
                focused_entity: press.entity,
                window,
            });
        }
    }
}

/// Plugin which focuses (or blurs) entities in response to pointer clicks.
///
/// On a pointer press this hides the focus indicator ([`InputFocusVisible`]) and triggers a
/// bubbling [`AcquireFocus`] on the clicked entity.
/// [`InputFocusPlugin`](crate::InputFocusPlugin) resolves that request by focusing the first
/// [`Focusable`](crate::Focusable) ancestor, or clearing focus when it reaches the window.
///
/// Because that [`AcquireFocus`] event is shared across widgets and focus
/// schemes, be deliberate when changing anything in this pathway — a change here can have
/// engine-wide focus consequences. Individual widgets may also intercept the event and stop its
/// propagation to implement custom behavior (e.g. the number-input scrubber focuses on pointer
/// *release* rather than press). See the docs on [`on_window_acquire_focus_clear`](crate::on_window_acquire_focus_clear) and
/// [`crate::acquire_focus`].
///
/// This is intentionally independent of any navigation scheme: it works with tab navigation,
/// directional navigation, or on its own (e.g. an app with only text input). This whole module is
/// gated behind the `bevy_picking` feature.
///
/// [`TabNavigationPlugin`]: crate::tab_navigation::TabNavigationPlugin
pub struct PointerFocusPlugin;

impl Plugin for PointerFocusPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(click_to_focus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AcquireFocus, InputFocus, InputFocusPlugin};
    use bevy_app::App;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_input::InputPlugin;
    use bevy_window::{PrimaryWindow, Window};

    /// Sets up an app with a primary window and the input-focus + pointer-focus plugins, but
    /// deliberately *without* any navigation plugin (no tab navigation), with initial focus resolved.
    fn pointer_focus_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputFocusPlugin, PointerFocusPlugin));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        // Resolve initial focus (focus goes to the primary window).
        app.update();
        (app, window)
    }

    /// With no tab navigation installed, an `AcquireFocus` on a non-focusable entity must still
    /// bubble up to the window and clear focus. This results in "click outside to unfocus" behavior.
    #[test]
    fn acquire_focus_without_tab_navigation_clears_focus_at_window() {
        let (mut app, window) = pointer_focus_app();

        // Start with some entity focused.
        let previously_focused = app.world_mut().spawn_empty().id();
        app.world_mut()
            .insert_resource(InputFocus::from_entity(previously_focused));

        // Click away onto a non-focusable child of the window.
        let non_focusable = app.world_mut().spawn(ChildOf(window)).id();
        app.world_mut().trigger(AcquireFocus {
            focused_entity: non_focusable,
            window,
        });
        app.update();

        assert_eq!(app.world().resource::<InputFocus>().get(), None);
    }
}
