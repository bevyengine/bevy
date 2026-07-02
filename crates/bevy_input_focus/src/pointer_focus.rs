//! Pointer-driven focus: focus or blur entities in response to mouse/pointer clicks.
//!
//! This module is intentionally independent of any focus-navigation scheme (such as
//! [`tab_navigation`](crate::tab_navigation) or
//! [`directional_navigation`](crate::directional_navigation)). It only *requests* a focus change
//! by triggering a bubbling [`AcquireFocus`](crate::AcquireFocus) event on the clicked entity;
//! how that request is resolved depends on which focus observers are installed.
//!
//! Add [`PointerFocusPlugin`] to enable it. Requires the `bevy_picking` feature.

use bevy_app::{App, Plugin};

#[cfg(feature = "bevy_picking")]
use bevy_ecs::{
    entity::Entity,
    observer::On,
    query::With,
    system::{Commands, Query, ResMut},
};
#[cfg(feature = "bevy_picking")]
use bevy_window::PrimaryWindow;

#[cfg(feature = "bevy_picking")]
use crate::{AcquireFocus, InputFocusVisible};

/// Observer which requests focus for a clicked entity.
///
/// On a pointer press this hides the focus indicator ([`InputFocusVisible`]) and triggers a
/// bubbling [`AcquireFocus`] on the clicked entity. It does not itself decide *what* becomes
/// focused — that is the job of whatever [`AcquireFocus`] resolver observers are installed (see
/// [`PointerFocusPlugin`]).
#[cfg(feature = "bevy_picking")]
fn click_to_focus(
    press: On<bevy_picking::events::Pointer<bevy_picking::events::Press>>,
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
/// bubbling [`AcquireFocus`](crate::AcquireFocus) on the clicked entity. That request is then
/// resolved by whatever focus observers are installed:
/// - a focus-target observer such as
///   [`acquire_focus_tab_index`](crate::tab_navigation::acquire_focus_tab_index) focuses the
///   target if it is focusable, or
/// - the generalized [`acquire_focus`](crate::acquire_focus) observer clears focus once the
///   request bubbles up to the window (this is "click outside to unfocus").
///
/// Because that [`AcquireFocus`](crate::AcquireFocus) event is shared across widgets and focus
/// schemes, be deliberate when changing anything in this pathway — a change here can have
/// engine-wide focus consequences. Individual widgets may also intercept the event and stop its
/// propagation to implement custom behavior (e.g. the number-input scrubber focuses on pointer
/// *release* rather than press). See the docs on [`acquire_focus`](crate::acquire_focus) and
/// [`acquire_focus_tab_index`](crate::tab_navigation::acquire_focus_tab_index).
///
/// This is intentionally independent of any navigation scheme: it works with tab navigation,
/// directional navigation, or on its own (e.g. an app with only text input). Requires the
/// `bevy_picking` feature; without it this plugin is a no-op.
pub struct PointerFocusPlugin;

impl Plugin for PointerFocusPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_picking")]
        app.add_observer(click_to_focus);
        #[cfg(not(feature = "bevy_picking"))]
        let _ = app;
    }
}

#[cfg(all(test, feature = "bevy_picking"))]
mod tests {
    use super::*;
    use crate::{AcquireFocus, InputFocus, InputFocusPlugin};
    use bevy_app::App;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_input::InputPlugin;
    use bevy_window::{PrimaryWindow, Window};

    /// Sets up an app with a primary window and the input-focus + pointer-focus plugins, but
    /// deliberately *without* any navigation plugin (no tab navigation), with initial focus
    /// resolved.
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
    /// bubble up to the window and clear focus. This is the "click outside to unfocus" behavior
    /// that #24695 requires to work independently of `TabNavigationPlugin`.
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
