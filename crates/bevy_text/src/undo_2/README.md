# Undo done the right way

This code is from the library `undo_2` , which has a version `0.3.0` that is not released to `crates.io`.

[Original source](https://gitlab.com/okannen/undo_2/-/tree/b32c34edb2c15c266b946f0d82188624f3aa3fdc)

It is temporarily upstreamed to this repo, with the intention to use the original source in the future.

## Introduction

An undo crate that makes it so that, the instant you edit something you undid
to, instead of truncating the undo/redo history, it bakes the rewind onto the
end of the Undo history as a precursor to your new change. I found the idea in
[zaboople/klong][zaboople]. This crate is an implementation
of this idea with a minor variation explained below.

As an example consider the following sequence of commands:

| Command | State |
| ------- | ----- |
| Init    |       |
| Do A    | A     |
| Do B    | A, B  |
| Undo    | A     |
| Do C    | A, C  |

With the **classical undo**, repeated undo would lead to the sequence:

| Command | State |
|---------|-------|
|         | A, C  |
| Undo    | A     |
| Undo    |       |

Starting from 5, with **undo_2**, repeating undo would lead to the sequence:

| Command | State |
|---------|-------|
|         | A, C  |
| Undo    | A     |
| Undo    | A,B   |
| Undo    | A     |
| Undo    |       |

**undo_2**'s undo navigates back in history, while classical undo navigates back
through the sequence of command that builds the state.

This is actualy the way undo is often implemented in mac's (cocoa library), emacs
and it is similar to vim :earlier.

## Features

  1. historical undo sequence, no commands are lost.
  2. user-friendly compared to complex undo trees.
  3. optimized implementation: no commands are ever copied.
  4. very lightweight, dumb and simple.
  5. possibility to merge and splice commands.

## How to use it

Add the dependency to the cargo file:

```toml
[dependencies]
undo_2 = "0.1"
```

Then add this to your source file:

```ignore
use undo_2::Commands;
```

The example below implements a dumb text editor. *undo_2* does not perform
itself "undos" and "redos", rather, it returns a sequence of commands that must
be interpreted by the application. This design pattern makes implementation
easier because it is not necessary to borrow data within the stored list of
commands.

```rs
use undo_2::{Commands,Action};
use Action::{Do,Undo};

enum Command {
    Add(char),
    Delete(char),
}

struct TextEditor {
    text: String,
    command: Commands<Command>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            command: Commands::new(),
        }
    }
    pub fn add_char(&mut self, c: char) {
        self.text.push(c);
        self.command.push(Command::Add(c));
    }
    pub fn delete_char(&mut self) {
        if let Some(c) = self.text.pop() {
            self.command.push(Command::Delete(c));
        }
    }
    pub fn undo(&mut self) {
        for action in self.command.undo() {
            interpret_action(&mut self.text, action)
        }
    }
    pub fn redo(&mut self) {
        for action in self.command.redo() {
            interpret_action(&mut self.text, action)
        }
    }
}

fn interpret_action(data: &mut String, action: Action<&Command>) {
    use Command::*;
    match action {
        Do(Add(c)) | Undo(Delete(c)) => {
            data.push(*c);
        }
        Undo(Add(_)) | Do(Delete(_)) => {
            data.pop();
        }
    }
}

let mut editor = TextEditor::new();
editor.add_char('a'); //              :[1]
editor.add_char('b'); //              :[2]
editor.add_char('d'); //              :[3]
assert_eq!(editor.text, "abd");

editor.undo(); // first undo          :[4]
assert_eq!(editor.text, "ab");

editor.add_char('c'); //              :[5]
assert_eq!(editor.text, "abc");

editor.undo(); // Undo [5]            :[6]
assert_eq!(editor.text, "ab");
editor.undo(); // Undo the undo [4]   :[7]
assert_eq!(editor.text, "abd");
editor.undo(); // Undo [3]            :[8]
assert_eq!(editor.text, "ab");
editor.undo(); // Undo [2]            :[9]
assert_eq!(editor.text, "a");
```

## More information

1. After a sequence of consecutive undo, if a new command is added, the undos
   forming the sequence are merged. This makes the traversal of the undo
   sequence more concise by avoiding state duplication.

| Command | State   | Comment              |
|---------|-------  |----------------------|
| Init    |         |                      |
| Do A    | A       |                      |
| Do B    | A,B     |                      |
| Do C    | A, B, C |                      |
| Undo    | A, B    |Merged                |
| Undo    | A       |Merged                |
| Do D    | A, D    |                      |
| Undo    | A       |Redo the 2 Merged Undo|
| Undo    | A, B, C |                      |
| Undo    | A, B    |                      |
| Undo    | A       |                      |
| Undo    |         |                      |

1. Each execution of an undos or redo may lead to the execution of a sequence of
   actions in the form `Undo(a)+Do(b)+Do(c)`. Basic arithmetic is implemented
   assuming that `Do(a)+Undo(a)` is equivalent to not doing anything (here the
   2 `a`'s designate the same entity, not to equal objects).

The piece of code below, which is the longer version of the code above, illustrates points 1 and 2.

```ignore
let mut editor = TextEditor::new();
editor.add_char('a'); //              :[1]
editor.add_char('b'); //              :[2]
editor.add_char('d'); //              :[3]
assert_eq!(editor.text, "abd");

editor.undo(); // first undo          :[4]
assert_eq!(editor.text, "ab");

editor.add_char('c'); //              :[5]
assert_eq!(editor.text, "abc");

editor.undo(); // Undo [5]            :[6]
assert_eq!(editor.text, "ab");
editor.undo(); // Undo the undo [4]   :[7]
assert_eq!(editor.text, "abd");
editor.undo(); // Undo [3]            :[8]
assert_eq!(editor.text, "ab");
editor.undo(); // Undo [2]            :[9]
assert_eq!(editor.text, "a");

editor.add_char('z'); //              :[10]
assert_eq!(editor.text, "az");
// when an action is performed after a sequence
// of undo, the undos are merged: undos [6] to [9] are merged now

editor.undo(); // back to [10]
assert_eq!(editor.text, "a");
editor.undo(); // back to [5]: reverses the consecutive sequence of undos in batch
assert_eq!(editor.text, "abc");
editor.undo(); // back to [4]
assert_eq!(editor.text, "ab");
editor.undo(); // back to [3]
assert_eq!(editor.text, "abd");
editor.undo(); // back to [2]
assert_eq!(editor.text, "ab");
editor.undo(); // back to [1]
assert_eq!(editor.text, "a");
editor.undo(); // back to [0]
assert_eq!(editor.text, "");

editor.redo(); // back to [1]
assert_eq!(editor.text, "a");
editor.redo(); // back to [2]
assert_eq!(editor.text, "ab");
editor.redo(); // back to [3]
assert_eq!(editor.text, "abd");
editor.redo(); // back to [4]
assert_eq!(editor.text, "ab");
editor.redo(); // back to [5]
assert_eq!(editor.text, "abc");
editor.redo(); // back to [9]: redo inner consecutive sequence of undos in batch
               //              (undo are merged only when they are not the last action)
assert_eq!(editor.text, "a");
editor.redo(); // back to [10]
assert_eq!(editor.text, "az");

editor.add_char('1');
editor.add_char('2');
assert_eq!(editor.text, "az12");
editor.undo();
editor.undo();
assert_eq!(editor.text, "az");
editor.redo(); // undo is the last action, undo the undo only once
assert_eq!(editor.text, "az1");
editor.redo();
assert_eq!(editor.text, "az12");
```

## Release note

### Version 0.3

- [`Action`] is now an enum taking commands, the list of command to be
executed is of the form [`Action<T>`];
- added [`Commands::can_undo`] and [`Commands::can_redo`];
- added [`Commands::rebuild`], which correspond to the classical redo;
- fixed a bug in [`Commands::undo_or_redo_to_index`]
- Added support for special commands that represent a state setting. See [`SetOrTransition`].

[zaboople]: https://github.com/zaboople/klonk/blob/master/TheGURQ.md
