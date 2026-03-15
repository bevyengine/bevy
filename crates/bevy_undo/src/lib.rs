//! A framework for managing undo/redo in Bevy apps.
//!
//! The key trait is [`UndoAction`], which represents a mutation to the application state.
//!
//! The framework supports independent undo contexts. Consider for example the case of a user
//! typing text into a code editor, and then pulling up the "find" dialog box. The find dialog
//! has its own separate undo/redo context that is separate from the file being edited.

extern crate alloc;

use alloc::borrow::Cow;
use bevy_ecs::{entity::Entity, resource::Resource, world::World};
use bevy_platform::collections::HashMap;
use core::any::Any;

/// Represents an action which can be undone. Applying the action will modify the state of the app,
/// undoing the changes.
///
/// Examples of undoable actions are:
/// * Typing some characters into a text input field.
/// * Painting pixels on a sprite
/// * Deleting part of a scene in a scene editor.
///
/// The details of how the undo actually works - whether by snapshotting or diffing - is up to
/// the trait implementer.
///
/// Multiple consecutive actions which are similar can be coalesced into a single action. For
/// example, typing a word in a text input field can be undone as a group, rather than having to
/// undo each individual keystroke.
pub trait UndoAction: Send + Sync + 'static {
    /// Apply the action. This undoes the modification to the app state. This consumes the action,
    /// and returns an inverse action, one that reverses the changes. So an "undo" yields a
    /// "redo" and vice versa.
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction>;

    /// Try to coalesce this action with the previous action on the stack. If successful, returns
    /// a new [`UndoAction`] that represents the combination of the two actions, which will replace
    /// the current top of stack; otherwise, no merge will occur and the new action will be pushed
    /// separately.
    fn coalesce(&self, prev: &dyn UndoAction) -> Option<Box<dyn UndoAction>>;

    /// Human-readable label, e.g. "Paste", "Delete". This may be displayed in the UI in a tooltip
    /// or menu label.
    fn label(&self) -> Cow<'static, str>;

    /// For downcasting; this is used when coalescing.
    fn as_any(&self) -> &dyn Any;
}

/// An id representing an undo context. Typically, there will be a context for the whole app,
/// and then separate contexts for input fields or dialogs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ContextId {
    /// The Entity of the element being edited.
    pub entity: Entity,
    /// A string tag to distinguish context kinds on the same entity.
    pub kind: Cow<'static, str>,
}

/// The resource which contains the root of all the undo stacks.
#[derive(Resource, Default)]
pub struct UndoStack {
    /// Stores all existing context stacks.
    stacks: HashMap<ContextId, ActionStack>,

    /// Indicates which stack is currently active.
    active: Option<ContextId>,
}

/// The undo / redo stack for a single undo context.
#[derive(Default)]
struct ActionStack {
    undo: Vec<Box<dyn UndoAction>>,
    redo: Vec<Box<dyn UndoAction>>,
}

impl UndoStack {
    /// Set the current undo context, and return the previous context. Typically this will
    /// be done when entering a new mode - for example, when a text input widget gains focus.
    pub fn activate_context(&mut self, ctx: ContextId) -> Option<ContextId> {
        let prev = self.active.take();
        self.stacks.entry(ctx.clone()).or_default();
        self.active = Some(ctx);
        prev
    }

    /// Clean up an existing undo context. If the context being removed is the active
    /// context, then the active context will be set to `None`.
    pub fn delete_context(&mut self, ctx: ContextId) {
        self.stacks.remove(&ctx);
        if self.active.as_ref() == Some(&ctx) {
            self.active = None;
        }
    }

    /// Remove the active undo context, and replace it with the given context. This would
    /// typically be called when exiting a UI mode - for example, when a text input field
    /// loses focus.
    pub fn replace_active_context(&mut self, ctx: ContextId) {
        if let Some(active_ctx) = self.active.take() {
            self.stacks.remove(&active_ctx);
        }
        self.stacks.entry(ctx.clone()).or_default();
        self.active = Some(ctx);
    }

    /// Push a new [`UndoAction`] on to the undo stack. This also clears the redo stack.
    /// This should be done whenever we make a change to the app's document state.
    pub fn push(&mut self, ctx: &ContextId, action: Box<dyn UndoAction>) {
        let stack = self.stacks.entry(ctx.clone()).or_default();
        // Attempt coalesce with top of undo stack
        if let Some(prev) = stack.undo.last()
            && let Some(merged) = action.coalesce(prev.as_ref())
        {
            stack.undo.pop();
            stack.undo.push(merged);
            stack.redo.clear();
            return;
        }
        stack.undo.push(action);
        stack.redo.clear();
    }

    /// Pop the most recent undo action and apply it. This will push the inverse
    /// action on the redo stack.
    pub fn undo(&mut self, world: &mut World) {
        let Some(ctx) = &self.active else { return };
        let Some(stack) = self.stacks.get_mut(ctx) else {
            return;
        };
        if let Some(action) = stack.undo.pop() {
            let redo = action.apply(world);
            stack.redo.push(redo);
        }
    }

    /// Pop the most recent redo action and apply it. This will push the inverse action on the
    /// undo stack.
    pub fn redo(&mut self, world: &mut World) {
        let Some(ctx) = &self.active else { return };
        let Some(stack) = self.stacks.get_mut(ctx) else {
            return;
        };
        if let Some(action) = stack.redo.pop() {
            let undo = action.apply(world);
            stack.undo.push(undo);
        }
    }

    /// Label for topmost undo item.
    pub fn undo_label(&self) -> Option<Cow<'static, str>> {
        let stack = self.stacks.get(self.active.as_ref()?)?;
        Some(stack.undo.last()?.label())
    }

    /// Label for topmost redo item.
    pub fn redo_label(&self) -> Option<Cow<'static, str>> {
        let stack = self.stacks.get(self.active.as_ref()?)?;
        Some(stack.redo.last()?.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyAction {
        label: &'static str,
        applied: bool,
    }

    impl UndoAction for DummyAction {
        fn apply(self: Box<Self>, _world: &mut World) -> Box<dyn UndoAction> {
            Box::new(DummyAction {
                label: self.label,
                applied: !self.applied,
            })
        }

        fn coalesce(&self, prev: &dyn UndoAction) -> Option<Box<dyn UndoAction>> {
            if let Some(prev) = prev.as_any().downcast_ref::<DummyAction>()
                && self.label == prev.label
            {
                return Some(Box::new(DummyAction {
                    label: self.label,
                    applied: self.applied || prev.applied,
                }));
            }
            None
        }

        fn label(&self) -> Cow<'static, str> {
            Cow::Borrowed(self.label)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn push_and_undo_redo() {
        let mut world = World::new();
        let ctx = ContextId {
            entity: world.spawn_empty().id(),
            kind: Cow::Borrowed("main"),
        };
        let mut stack = UndoStack::default();
        stack.activate_context(ctx.clone());

        stack.push(
            &ctx,
            Box::new(DummyAction {
                label: "Edit",
                applied: false,
            }),
        );
        assert_eq!(stack.undo_label().unwrap(), "Edit");
        assert!(stack.redo_label().is_none());

        stack.undo(&mut world);
        assert_eq!(stack.redo_label().unwrap(), "Edit");
        assert!(stack.undo_label().is_none());

        stack.redo(&mut world);
        assert_eq!(stack.undo_label().unwrap(), "Edit");
        assert!(stack.redo_label().is_none());
    }

    #[test]
    fn coalesce_actions() {
        let mut world = World::new();
        let ctx = ContextId {
            entity: world.spawn_empty().id(),
            kind: Cow::Borrowed("input"),
        };
        let mut stack = UndoStack::default();
        stack.activate_context(ctx.clone());

        stack.push(
            &ctx,
            Box::new(DummyAction {
                label: "Type",
                applied: false,
            }),
        );
        stack.push(
            &ctx,
            Box::new(DummyAction {
                label: "Type",
                applied: false,
            }),
        );

        let action_stack = stack.stacks.get(&ctx).unwrap();
        assert_eq!(action_stack.undo.len(), 1);
        assert_eq!(action_stack.undo.last().unwrap().label(), "Type");
    }

    #[test]
    fn multiple_contexts() {
        let mut world = World::new();
        let ctx1 = ContextId {
            entity: world.spawn_empty().id(),
            kind: Cow::Borrowed("main"),
        };
        let ctx2 = ContextId {
            entity: world.spawn_empty().id(),
            kind: Cow::Borrowed("dialog"),
        };
        let mut stack = UndoStack::default();

        stack.activate_context(ctx1.clone());
        stack.push(
            &ctx1,
            Box::new(DummyAction {
                label: "Edit",
                applied: false,
            }),
        );

        stack.activate_context(ctx2.clone());
        stack.push(
            &ctx2,
            Box::new(DummyAction {
                label: "Dialog",
                applied: false,
            }),
        );

        stack.active = Some(ctx1.clone());
        assert_eq!(stack.undo_label().unwrap(), "Edit");

        stack.active = Some(ctx2.clone());
        assert_eq!(stack.undo_label().unwrap(), "Dialog");
    }

    #[test]
    fn undo_redo_empty_stack() {
        let mut world = World::new();
        let ctx = ContextId {
            entity: world.spawn_empty().id(),
            kind: Cow::Borrowed("empty"),
        };
        let mut stack = UndoStack::default();
        stack.activate_context(ctx.clone());

        stack.undo(&mut world);
        stack.redo(&mut world);
        assert!(stack.undo_label().is_none());
        assert!(stack.redo_label().is_none());
    }
}
