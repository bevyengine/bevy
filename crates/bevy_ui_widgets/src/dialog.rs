use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    lifecycle::{Add, Remove},
    observer::On,
    query::{With, Without},
    reflect::{ReflectComponent, ReflectEvent},
    resource::Resource,
    schedule::{common_conditions::resource_changed, IntoScheduleConfigs},
    system::{Commands, Query, Res, ResMut, Single},
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_picking::{
    events::{Drag, DragStart, Pointer, Press},
    pointer::PointerButton,
};
use bevy_reflect::Reflect;
use bevy_ui::{GlobalZIndex, UiScale, UiTransform, Val2};
use bevy_window::{PrimaryWindow, Window};

use crate::ModalDialog;

/// A dialog box. When [`ModalDialog`] is also present it traps focus and is backed by a
/// [`crate::ModalDialogBarrier`]; when absent it is a movable, non-blocking floating window.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Dialog)))]
#[require(TabGroup { order: 0, modal: false })]
#[reflect(Component)]
pub struct Dialog;

/// Event used to indicate that the dialog wants to be closed. This can happen because
/// the user clicked on the barrier, hit the escape key, or clicked the close box in the dialog
/// title. This event propagates so that the owner of the dialog can despawn it.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
#[derive(Reflect)]
#[reflect(Event)]
pub struct RequestClose {
    /// The [`Dialog`] that triggered this event.
    #[event_target]
    pub source: Entity,
}

const FLOATING_Z_BASE: i32 = 50;
const FLOATING_Z_MAX: i32 = 89;

/// Marks a region (e.g. a title bar) that drags its owning [`Dialog`].
#[derive(Component, Debug, Reflect, Clone, Default)]
#[require(DialogDragState)]
#[reflect(Component)]
pub struct DialogDragHandle;

/// Records the owning dialog's translation when a drag begins, so each drag event
/// can set an absolute position from the drag's cumulative distance.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct DialogDragState {
    /// The owning dialog's `UiTransform.translation`
    start_dialog_translation: Val2,
    /// The starting location of the drag (logical coordinates)
    start_pointer_location: Vec2,
}

/// Open floating dialogs, ordered bottom-to-top; the front-most is last.
#[derive(Resource, Default)]
pub struct DialogStack(Vec<Entity>);

impl DialogStack {
    /// Add (or move) a dialog to the front of the stack.
    fn push_top(&mut self, entity: Entity) {
        self.0.retain(|&e| e != entity);
        self.0.push(entity);
    }

    /// Remove a dialog from the stack.
    fn remove(&mut self, entity: Entity) {
        self.0.retain(|&e| e != entity);
    }
}

/// Track newly-spawned floating dialogs at the top of the stack.
fn register_dialog(
    add: On<Add, Dialog>,
    q_dialog: Query<&Dialog, Without<ModalDialog>>,
    mut stack: ResMut<DialogStack>,
) {
    let entity = add.event_target();
    if q_dialog.contains(entity) {
        stack.push_top(entity);
    }
}

/// Drop dialogs from the stack when they despawn.
fn deregister_dialog(
    remove: On<Remove, Dialog>,
    q_dialog: Query<&Dialog, Without<ModalDialog>>,
    mut stack: ResMut<DialogStack>,
) {
    let entity = remove.event_target();
    if q_dialog.contains(entity) {
        stack.remove(entity);
    }
}

/// Give each floating dialog a `GlobalZIndex` from its stack position, so order
/// drives both draw order and pointer-pick order.
fn sync_dialog_z(stack: Res<DialogStack>, mut commands: Commands) {
    for (index, &entity) in stack.0.iter().enumerate() {
        let z = (FLOATING_Z_BASE + index as i32).min(FLOATING_Z_MAX);
        commands.entity(entity).insert(GlobalZIndex(z));
    }
}

/// Raise a floating dialog to the front of the stack when it is pressed.
fn bring_to_front(
    press: On<Pointer<Press>>,
    q_dialog: Query<&Dialog, Without<ModalDialog>>,
    q_parent: Query<&ChildOf>,
    mut stack: ResMut<DialogStack>,
) {
    let target = press.event_target();
    let Some(dialog) = core::iter::once(target)
        .chain(q_parent.iter_ancestors(target))
        .find(|&e| q_dialog.contains(e))
    else {
        return;
    };
    // Already on top.
    if stack.0.last() == Some(&dialog) {
        return;
    }
    stack.push_top(dialog);
}

/// Record the dialog's translation when a drag begins on its [`DialogDragHandle`].
fn dialog_drag_start(
    drag_start: On<Pointer<DragStart>>,
    q_dialog: Query<(), With<Dialog>>,
    q_parent: Query<&ChildOf>,
    q_transform: Query<&UiTransform>,
    mut q_state: Query<&mut DialogDragState>,
    ui_scale: Res<UiScale>,
) {
    if drag_start.button != PointerButton::Primary {
        return;
    }
    // Only the handle entity itself drives the move.
    let handle = drag_start.event_target();
    let Ok(mut state) = q_state.get_mut(handle) else {
        return;
    };
    let Some(dialog) = q_parent
        .iter_ancestors(handle)
        .find(|&e| q_dialog.contains(e))
    else {
        return;
    };

    if let Ok(translation) = q_transform.get(dialog).map(|t| t.translation) {
        state.start_dialog_translation = translation;
    } else {
        warn!("Cannot get translation from dialog for dragging");
        state.start_dialog_translation = Val2::ZERO;
    }
    state.start_pointer_location = drag_start.pointer_location.position / ui_scale.0;
}

/// Move a dialog by dragging its [`DialogDragHandle`], positioning it at the
/// drag-start translation plus the drag's cumulative distance.
fn dialog_drag(
    drag: On<Pointer<Drag>>,
    q_handle: Query<&DialogDragState, With<DialogDragHandle>>,
    q_dialog: Query<(), With<Dialog>>,
    q_parent: Query<&ChildOf>,
    mut q_transform: Query<&mut UiTransform>,
    ui_scale: Res<UiScale>,
    // TODO: multiple windows? dragging between them, etc
    primary_window: Single<&Window, With<PrimaryWindow>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let handle = drag.event_target();
    let Ok(state) = q_handle.get(handle) else {
        return;
    };
    let Some(dialog) = q_parent
        .iter_ancestors(handle)
        .find(|&e| q_dialog.contains(e))
    else {
        return;
    };

    const SCREEN_EDGE_MARGIN: f32 = 16.;

    // `distance` is in logical pixels; a `Val::Px` translation is scaled by
    // `UiScale`, so divide that out. Then clamp to the screen region using
    // the start of the pointer pos as an offset from screen bounds.
    let clamped_offset = (drag.distance / ui_scale.0).clamp(
        Vec2::splat(SCREEN_EDGE_MARGIN) - state.start_pointer_location,
        primary_window.size() - Vec2::splat(SCREEN_EDGE_MARGIN) - state.start_pointer_location,
    );
    if let Ok(mut transform) = q_transform.get_mut(dialog)
        && let Ok(translation) = state
            .start_dialog_translation
            .try_add(Val2::px(clamped_offset.x, clamped_offset.y))
    {
        transform.translation = translation;
    }
}

/// Plugin that adds the observers and systems for the [`Dialog`] widget.
pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogStack>()
            .add_observer(register_dialog)
            .add_observer(deregister_dialog)
            .add_observer(dialog_drag_start)
            .add_observer(dialog_drag)
            .add_observer(bring_to_front)
            .add_systems(
                Update,
                sync_dialog_z.run_if(resource_changed::<DialogStack>),
            );
    }
}
