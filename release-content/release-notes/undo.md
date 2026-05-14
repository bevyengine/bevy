---
title: "Undo / Redo Framework"
authors: ["@viridia"]
pull_requests: []
---

The new `bevy_undo_redo` crate provides a basic framework for managing undo and redo actions. This will
be a key feature of the Bevy editor, but it can also be used for third-party editors. Text input
fields will also be able to take advantage of this.

The key trait is [`UndoAction`], which represents a modification to the state of the app. The
trait makes no restriction on what kind of mutation is being undone, which means that future
editor plugins will be able to define their own custom undo actions.
