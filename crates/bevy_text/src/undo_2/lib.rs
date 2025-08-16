//! AA
#![doc = include_str!("./README.md")]
//! BB
// #![doc = document_features::document_features!()]
//! CC
use alloc::{slice, vec};
use core::borrow::Borrow;
use core::cmp::min;
use core::fmt::Debug;
use core::iter::FusedIterator;
use core::ops::ControlFlow;
use core::ops::{Deref, Index};
use core::option;
use core::slice::SliceIndex;
use std::hash::{DefaultHasher, Hash, Hasher as _};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Action that should be taken on the paired command
///
/// [`ActionIter`] item type is a pair `(Action, C)` where
/// C is the client defined command type. `(Action::Do, t)`
/// signifies that the command `t` shall be executed. `(Action::Undo,t)`
/// means it shall be undone.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Action<T> {
    /// The client shall do the paired command.
    Do(T),
    /// The client shall undo the paired command.
    Undo(T),
}

/// The commands in that are listed in [Commands] shall
/// represent state *transition* so that this transitions can
/// be done and undone. For exemple the state transition "add the letter 'a'
/// after the cursor" can be done by adding the letter 'a' and moving the cursor left,
/// or undone by executing a backspace.
///
/// Representing commands as *transitions* is necessary as commands executions will
/// depend of the current state of the program that is a concequence of the sequence
/// of execution of indetermined commands.
///
/// But sometimes, the transition affects a piece of state of the program which is
/// absolutely indepent of other state of the program and of which no other command depends. Lets
/// call such state *independent state*. This could be for exemple the position of the view port of the text editor. Moving the
/// position of the view port does not affect the edition. On the other hand, the current color
/// of the pencil can not be considered an *independent state* as edition commands depend on this
/// color.
///
/// For such *independ state* the resulting global state of the program is independent of the order
/// in which the transition to this state are applied, and it is independent of the way this
/// transition are interlieved with other transitions.
///
/// So the set of all commands can be partitionned in two: the set of transitions affecting
/// indepent state, and the other transitions. [`SetOrTransition`] provides such a partition.
/// It is supposed to be used as the element of the [Commands]:
/// `Commands<SetOrTransition::<MyStateCommands,MyTransitions>`.
///
/// [Commands] provides dedicateds methods (see [`apply_actions`][Commands::apply_actions]) to simplify the use of
/// commands partitionned in independent state and transitions.
///
/// State commands are supposed to represent the setting of an indepent state such as "set the view
/// port to `x`, `y`, `lx`, `ly`". When a independent state command is undone within the `apply_actions`, the
/// algorithm will look for the previous application of a command the same key [`IndepStateKey::key`] and will
/// command the application of it.
///
/// # Example
///
/// In this example we use commands that set the state `color` and `length`.
///
/// In the commands registered the previous states of `color` and `length` are not stored
/// but [`Commands::apply_actions`] will retrieve automatically the value to set for this states.
/// ```
/// use std::mem::{discriminant, Discriminant};
///
/// use bevy_text::undo_2::{Commands, IndepStateKey, SetOrTransition, SetTransAction};
///
/// #[derive(Copy, Clone, Debug)]
/// struct State {
///     color: f64,
///     length: f64,
/// }
/// impl State {
///     fn new() -> Self {
///         INIT_STATE
///     }
///     fn apply_set(&mut self, c: &SetCommands) {
///         match c {
///             SetCommands::Color(v) => self.color = *v,
///             SetCommands::Length(v) => self.length = *v,
///         };
///     }
///     fn execute_action(&mut self, c: SetTransAction<SetCommands, TransitionCommand>) {
///         match c {
///             SetTransAction::Do(_) => {}
///             SetTransAction::Undo(_) => {}
///             SetTransAction::Set(c) => self.apply_set(c),
///             SetTransAction::SetToInitial(d) => self.apply_set(SetCommands::new_initial(d)),
///         }
///     }
/// }
/// static INIT_STATE: State = State {
///     color: 0.,
///     length: 0.,
/// };
///
/// #[derive(Debug, Copy, Clone, PartialEq)]
/// enum SetCommands {
///     Color(f64),
///     Length(f64),
/// }
/// static INIT_COLOR: SetCommands = SetCommands::Color(INIT_STATE.color);
/// static INIT_LENGTH: SetCommands = SetCommands::Length(INIT_STATE.length);
///
/// impl SetCommands {
///     fn new_initial(d: Discriminant<Self>) -> &'static Self {
///         if d == discriminant(&INIT_COLOR) {
///             &INIT_COLOR
///         } else if d == discriminant(&INIT_LENGTH) {
///             &INIT_LENGTH
///         } else {
///             unreachable!("SetCommands::new_initial is not exhaustive: please, adds lacking initial value to this method")
///         }
///     }
/// }
///
/// impl IndepStateKey for SetCommands {
///     type KeyType = Discriminant<Self>;
///
///     fn key(&self) -> Self::KeyType {
///         discriminant(self)
///     }
/// }
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// enum TransitionCommand {
///     A,
///     B,
/// }
///
/// let mut commands = Commands::new();
/// let mut state = State::new();
///
/// let c = SetCommands::Color(1.);
/// state.apply_set(&c);
/// commands.push(SetOrTransition::Set(c));
///
/// commands.push(SetOrTransition::Transition(TransitionCommand::A));
/// commands.push(SetOrTransition::Transition(TransitionCommand::B));
///
/// let c = SetCommands::Length(10.);
/// state.apply_set(&c);
/// commands.push(SetOrTransition::Set(c));
///
/// let c = SetCommands::Color(2.);
/// state.apply_set(&c);
/// commands.push(SetOrTransition::Set(c));
///
/// commands.apply_undo(|c| {
///     assert_eq!(c, SetTransAction::Set(&SetCommands::Color(1.)));
///     state.execute_action(c);
/// });
/// assert_eq!(state.color, 1.);
/// assert_eq!(state.length, 10.);
/// commands.apply_redo(|c| {
///     assert_eq!(c, SetTransAction::Set(&SetCommands::Color(2.)));
///     state.execute_action(c);
/// });
/// assert_eq!(state.color, 2.);
/// assert_eq!(state.length, 10.);
///```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SetOrTransition<S, T> {
    /// A command that represent the setting of an independent state
    Set(S),
    /// Other commands
    Transition(T),
}
/// A key discriminating the different type of
/// independent state commands. For most enum,
/// it should `KeyType=std::mem::Discriminant<Self>`
/// and the key is generated by `std::mem::discriminant(&self)`
///
/// # Exemple
/// ```ignore
/// enum SetCommands{
///     ViewPort{x:f64,y:f64,lx:f64,ly:f64},
///     HideComment(bool),
/// }
///
/// impl IndepStateKey for SetCommands {
///     type KeyType = std::mem::Discriminant<Self>
///     fn key(&self) -> Self::KeyType {
///        std::mem::discriminant(self)
///     }
/// }
/// ```
pub trait IndepStateKey {
    /// stub `KeyType`
    type KeyType: PartialEq + Hash;
    /// stub key
    fn key(&self) -> Self::KeyType;
}

/// The actions asked to be performed by the user
/// when calling [`Commands::apply_actions`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SetTransAction<'a, S: IndepStateKey, T> {
    /// Ask the execution of the transition .0
    Do(&'a T),
    /// Ask the reverse of the execution of the transition .0
    Undo(&'a T),
    /// Ask to set state to .0
    Set(&'a S),
    /// Ask to set the state corresponding to the key .0 to its initial
    /// value: the value that it has before any command where applied to
    /// it
    SetToInitial(S::KeyType),
}

impl<T> Action<T> {
    /// stub `as_inner`
    pub fn as_inner(&self) -> &T {
        match self {
            Action::Do(a) | Action::Undo(a) => a,
        }
    }
    /// stub `as_inner_mut`
    pub fn as_inner_mut(&mut self) -> &mut T {
        match self {
            Action::Do(a) | Action::Undo(a) => a,
        }
    }
    /// stub `into_inner`
    pub fn into_inner(self) -> T {
        match self {
            Action::Do(a) | Action::Undo(a) => a,
        }
    }
}

/// The items stored in [Commands].
///
/// The list of `CommandItem` is accessible by dereferencing
/// the command list.
///
/// *NB*: The value inside the Undo variant is the number
/// of time the undo command is repeated minus 1.
///
/// # Example
///
/// ```
/// use bevy_text::undo_2::{Commands,CommandItem};
///
/// let mut commands = Commands::new();
///
/// #[derive(Debug,PartialEq)]
/// struct A;
///
/// commands.push(A);
/// commands.undo();
///
/// assert_eq!(*commands, [CommandItem::Command(A),CommandItem::Undo(0)]);
///
/// use CommandItem::Undo;
/// assert_eq!(*commands, [A.into(), Undo(0)]);
///
/// commands.push(A);
/// commands.undo();
/// commands.undo();
/// assert_eq!(*commands, [A.into(), Undo(0),A.into(),Undo(1)]);
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CommandItem<T> {
    /// A command typically created by [`Commands::push`](Commands#method.push)
    Command(T),
    /// Signify that `count` `CommandItem` previous to this item are undone.
    ///
    /// Where `count` refers to this variant field.
    Undo(usize),
}

/// Owns a slice of [commands](CommandItem) to undo and redo.
///
/// Commands are added by invoking [`push`](#method.push) method. [`undo`](#method.undo) and
/// [`redo`](#method.redo) return a list of [`Action<T>`] that the application must execute.
///
/// To see a full functional example, read [How to use it](index.html#how-to-use-it).
///
/// # Example
/// ```
/// use bevy_text::undo_2::{Action, Commands};
///
/// #[derive(Debug, Eq, PartialEq)]
/// enum Command {
///     A,
///     B,
/// }
/// use Command::*;
/// use Action::*;
///
/// let mut commands = Commands::new();
///
/// commands.push(A);
/// commands.push(B);
///
/// let v: Vec<_> = commands.undo().collect();
/// assert_eq!(v, [Undo(&B)]);
///
/// let v: Vec<_> = commands.undo().collect();
/// assert_eq!(v, [Undo(&A)]);
///
/// commands.push(A);
///
/// let v: Vec<_> = commands.undo().collect();
/// assert_eq!(v, [Undo(&A)]);
///
/// // undo the first 2 undos
/// let v: Vec<_> = commands.undo().collect();
/// assert_eq!(v, [Do(&A), Do(&B)]);
/// ```
///
/// # Representation
///
/// `Commands` owns a slice of [`CommandItem`] that is accesible
/// by dereferencing the command.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Eq)]
pub struct Commands<T> {
    commands: Vec<CommandItem<T>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    undo_cache: Vec<IndexedAction>,
}
impl<T: Debug> Debug for Commands<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Commands")
            .field("commands", &self.commands)
            .finish()
    }
}
impl<T: PartialEq> PartialEq for Commands<T> {
    fn eq(&self, other: &Self) -> bool {
        self.commands == other.commands
    }
}
impl<T: Hash> Hash for Commands<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.commands.hash(state);
    }
}
impl<T> Default for Commands<T> {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            undo_cache: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum CachedAction {
    Do,
    Undo,
    Skip,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct IndexedAction {
    action: CachedAction,
    index: usize,
}

/// Specify a merge when calling [`Commands::merge`](Commands#method.merge)
///
/// The [`end`, `start`) bounds the slice of command that will
/// be removed during the merge. `end` and `start` are in reverse order
/// because [`IterRealized`] goes backward.
///
/// If `command` is `None` then the slice will be removed, otherwise if
/// the `command` is `Some(c)` the slice will be replace by `c`.
#[derive(Debug)]
pub struct Merge<'a, T> {
    /// start
    pub start: IterRealized<'a, T>,
    /// end
    pub end: IterRealized<'a, T>,
    /// command
    pub command: Option<T>,
}

/// Specify a splice when calling [`Commands::splice`](Commands#method.splice)
///
/// The [`end`, `start`) bounds the slice of command that will
/// be removed during the merge. `end` and `start` are in reverse order
/// because [`IterRealized`] goes backward.
///
/// The removed slice is then replaced by the sequence (not reversed) of
/// commands denoted by `commands`.
#[derive(Debug)]
pub struct Splice<'a, T, I: IntoIterator<Item = T>> {
    /// start
    pub start: IterRealized<'a, T>,
    /// end
    pub end: IterRealized<'a, T>,
    /// commands
    pub commands: I,
}

#[derive(Debug)]
/// Iterator of actions returned by [`Commands::undo`](Commands#method.undo) and
/// [`Commands::redo`](Commands#method.redo)
pub struct ActionIter<'a, T> {
    commands: &'a [CommandItem<T>],
    to_do: slice::Iter<'a, IndexedAction>,
}
impl<T> Clone for ActionIter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            commands: self.commands,
            to_do: self.to_do.clone(),
        }
    }
}

#[derive(Debug)]
/// The type of the iterator returned by [`Commands::iter_realized`](Commands#method.iter_realized).
pub struct IterRealized<'a, T> {
    commands: &'a [CommandItem<T>],
    current: usize,
}
impl<T> Clone for IterRealized<'_, T> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

impl<S: IndepStateKey, T> Commands<SetOrTransition<S, T>> {
    /// Apply a series of actions represented by the type [`SetTransAction`] executed
    /// by argument `f`.
    ///
    /// The list of action to be applied are provided by the call of `act` on
    /// this [Commands]. This may be defined, for exemple as, `|c| c.redo()`
    ///
    /// This command only exist when the commands have the type [`SetOrTransition`].
    /// See the documentation of this type for an explanation.
    ///
    /// The function `f` will be called with [Do][SetTransAction::Do] and [Undo][SetTransAction::Undo] of commands that are
    /// represented as [transitions][SetOrTransition::Transition], and [Set][SetTransAction::Set] or [`SetToInitial`][SetTransAction::SetToInitial]
    /// (most probably only once) per [key][IndepStateKey::key] for commands that represent independent state setting.
    pub fn apply_actions(
        &mut self,
        act: impl FnOnce(&mut Self) -> ActionIter<'_, SetOrTransition<S, T>>,
        mut f: impl FnMut(SetTransAction<S, T>),
    ) {
        let mut state_commands = Vec::new();
        for command in act(self) {
            Self::apply_action(command, &mut state_commands, &mut f);
        }
        self.restore_state(state_commands, f);
    }
    /// Equivalent to `apply_actions(|c| c.undo(),f)`.
    pub fn apply_undo(&mut self, f: impl FnMut(SetTransAction<S, T>)) {
        self.apply_actions(Commands::undo, f);
    }
    /// Equivalent to `apply_actions(|c| c.redo(),f)`.
    pub fn apply_redo(&mut self, f: impl FnMut(SetTransAction<S, T>)) {
        self.apply_actions(Commands::redo, f);
    }
    fn apply_action(
        action: Action<&SetOrTransition<S, T>>,
        state_keys: &mut Vec<Option<S::KeyType>>,
        mut f: impl FnMut(SetTransAction<S, T>),
    ) {
        match action {
            Action::Do(SetOrTransition::Transition(tr)) => f(SetTransAction::Do(tr)),
            Action::Undo(SetOrTransition::Transition(tr)) => f(SetTransAction::Undo(tr)),
            Action::Do(SetOrTransition::Set(s)) | Action::Undo(SetOrTransition::Set(s)) => {
                state_keys.push(Some(s.key()));
            }
        }
    }
    fn restore_state(
        &self,
        mut state_keys: Vec<Option<S::KeyType>>,
        mut f: impl FnMut(SetTransAction<S, T>),
    ) {
        state_keys.sort_by_key(|v| {
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            hasher.finish()
        });
        state_keys.dedup();
        let mut l = state_keys.len();
        if l == 0 {
            return;
        }
        for command in self.iter_realized() {
            if let SetOrTransition::Set(st) = command {
                let st_key = st.key();
                if let Some(disc) =
                    state_keys.iter_mut().find(
                        |v| {
                            if let Some(d) = v {
                                *d == st_key
                            } else {
                                false
                            }
                        },
                    )
                {
                    *disc = None;
                    f(SetTransAction::Set(st));
                    l -= 1;
                    if l == 0 {
                        break;
                    }
                }
            }
        }
        if l > 0 {
            for disc in state_keys.into_iter().flatten() {
                f(SetTransAction::SetToInitial(disc));
            }
        }
    }
}

impl<T> Commands<T> {
    /// Create a new empty command sequence of type `T`.
    pub fn new() -> Self {
        Self {
            commands: vec![],
            undo_cache: vec![],
        }
    }
    /// The capacity of the underlying storage
    pub fn capacity(&self) -> usize {
        self.commands.capacity()
    }
    /// Reserve space for new commands
    pub fn reserve(&mut self, additional: usize) {
        self.commands.reserve(additional);
    }

    /// Return a reference to the last command
    /// if it is not an Undo.
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    ///
    /// assert_eq!(commands.last_command().unwrap(), &A);
    /// assert_eq!(commands.last().unwrap(), &CommandItem::Command(A));
    ///
    /// commands.undo();
    /// assert!(commands.last_command().is_none());
    /// ```
    pub fn last_command(&self) -> Option<&T> {
        self.commands.last().and_then(|v| match v {
            CommandItem::Command(c) => Some(c),
            CommandItem::Undo(_) => None,
        })
    }

    /// Return a mutable reference to the last command
    /// if it is not an Undo.
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    ///
    /// *commands.last_command_mut().unwrap() = Command::B;
    ///
    /// assert_eq!(commands.last_command().unwrap(), &B);
    /// assert_eq!(commands.last().unwrap(), &CommandItem::Command(B));
    ///
    /// commands.undo();
    /// assert!(commands.last_command_mut().is_none());
    /// ```
    pub fn last_command_mut(&mut self) -> Option<&mut T> {
        self.commands.last_mut().and_then(|v| match v {
            CommandItem::Command(c) => Some(c),
            CommandItem::Undo(_) => None,
        })
    }
    /// Change in place the last command.
    /// The update mail fail.
    ///
    /// It returns true if the update was possible and false otherwise.
    ///
    /// If the last command is accessible for modification, `updater`
    /// will receive this last command to update this last command. If
    /// updater shall return true if it modify the command and false otherwise
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A(usize),
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A(0));
    ///
    /// let updater = |a:&mut Command| if let A(i) = a {
    ///      *i+=1;
    ///      true
    ///   } else {
    ///      false
    ///   };
    ///
    /// assert_eq!(commands.last_command().unwrap(), &A(0));
    /// assert!(commands.update_last(updater));
    /// assert_eq!(commands.last_command().unwrap(), &A(1));
    /// commands.undo();
    /// assert!(!commands.update_last(updater));
    /// commands.push(B);
    /// assert!(!commands.update_last(updater));
    /// ```
    pub fn update_last(&mut self, updater: impl FnOnce(&mut T) -> bool) -> bool {
        if let Some(c) = self.last_command_mut() {
            updater(c)
        } else {
            false
        }
    }
    /// Change in place the last command or push a new command
    ///
    /// If the last command is accessible for modification, `updater`
    /// will receive this last command to update this last command. Otherwise
    /// it will receive none.
    ///
    /// If updater return `Some`, the value returned is then pushed in the command list.
    ///
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A(usize),
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A(0));
    ///
    /// let updater = |a:&mut Command| if let A(i) = a {
    ///      *i=10;
    ///      true
    ///   } else {
    ///     false
    ///   };
    /// let producer = || A(10);
    ///
    /// assert_eq!(commands.last_command().unwrap(), &A(0));
    /// commands.update_last_or_push(updater, producer);
    /// assert_eq!(commands.last_command().unwrap(), &A(10));
    /// commands.undo();
    /// commands.update_last_or_push(updater, producer);
    /// assert_eq!(commands.last_command().unwrap(), &A(10));
    /// commands.push(B);
    /// commands.update_last_or_push(updater, producer);
    /// assert_eq!(commands.last_command().unwrap(), &A(10));
    /// ```
    pub fn update_last_or_push(
        &mut self,
        updater: impl FnOnce(&mut T) -> bool,
        producer: impl FnOnce() -> T,
    ) {
        if !self.update_last(updater) {
            self.push(producer());
        }
    }

    /// Add the command `T`
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&A]);
    /// ```
    pub fn push(&mut self, command: T) {
        self.commands.push(CommandItem::Command(command));
    }

    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// undo.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Undo(&B)]);
    ///
    /// commands.push(C);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Undo(&C)]);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Do(&B)]);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Undo(&B)]);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Undo(&A)]);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert!(v.is_empty())
    /// ```
    #[must_use = "the returned actions should be realized"]
    pub fn undo(&mut self) -> ActionIter<'_, T> {
        self.undo_repeat(1)
    }
    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// undo `repeat` time.
    ///
    /// `undo_repeat(1)` is equivalent to `undo()`
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    ///
    /// let v: Vec<_> = commands.undo().collect();
    /// assert_eq!(v, [Undo(&B)]);
    ///
    /// commands.push(C);
    ///
    /// let v: Vec<_> = commands.undo_repeat(4).collect();
    /// assert_eq!(v, [Undo(&C), Undo(&A)]);
    /// ```
    #[must_use = "the returned actions should be realized"]
    pub fn undo_repeat(&mut self, repeat: usize) -> ActionIter<'_, T> {
        let Some(repeat) = repeat.checked_sub(1) else {
            return ActionIter::new();
        };
        let l = self.len();
        match self.commands.last_mut() {
            Some(CommandItem::Command(_)) => {
                let count = min(repeat, l - 1);
                self.commands.push(CommandItem::Undo(count));
                ActionIter::undo_at_count(
                    &self.commands,
                    &mut self.undo_cache,
                    l - 1 - count,
                    count,
                )
            }
            Some(CommandItem::Undo(i)) => {
                if *i + 2 < l {
                    let count = min(l - *i - 3, repeat);
                    *i = *i + 1 + count;
                    let t = l - *i - 2;
                    ActionIter::undo_at_count(&self.commands, &mut self.undo_cache, t, count)
                } else {
                    ActionIter::new()
                }
            }
            None => ActionIter::new(),
        }
    }
    /// An undo that skip undo branches.
    ///
    /// It returns the command that must be undone.
    ///
    /// It is equivalent to multiple successive call to `undo`. It behaves
    /// as a classical undo.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands,Action};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    /// commands.undo();
    /// commands.push(C);
    ///
    /// let c: Vec<_> = commands.unbuild().collect();
    /// assert_eq!(c, &[Undo(&C)]);
    ///
    /// let c: Vec<_> = commands.unbuild().collect();
    /// assert_eq!(c, &[Undo(&A)]);
    ///
    /// let c: Vec<_> = commands.unbuild().collect();
    /// assert!(c.is_empty());
    /// ```
    #[must_use = "the returned command should be undone"]
    pub fn unbuild(&mut self) -> ActionIter<'_, T> {
        let mut it = self.iter_realized();
        if it.next().is_none() {
            return ActionIter::new();
        }
        let start = it.index();
        let to_undo = if it.next().is_some() {
            start - it.index()
        } else {
            start + 1
        };
        self.undo_repeat(to_undo)
    }
    /// rebuild
    #[must_use = "the returned command should be undone"]
    pub fn rebuild(&mut self) -> ActionIter<'_, T> {
        if !self.is_undoing() {
            return ActionIter::new();
        }
        let l = self.commands.len();
        let mut it = IterRealized {
            commands: &self.commands[..l - 1],
            current: l - 1,
        };

        let mut prev_i = l - 1;
        match self.current_command_index() {
            Some(cur_i) => {
                while it.next().is_some() {
                    let n_i = it.index();
                    if n_i <= cur_i {
                        break;
                    }
                    prev_i = n_i;
                }
            }
            None => {
                while it.next().is_some() {
                    prev_i = it.index();
                }
            }
        }
        self.undo_or_redo_to_index(prev_i)
    }
    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// redo.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    /// commands.undo();
    /// let v: Vec<_> = commands.redo().collect();
    ///
    /// assert_eq!(v, [Do(&B)]);
    /// ```
    #[must_use = "the returned actions should be realized"]
    pub fn redo(&mut self) -> ActionIter<'_, T> {
        self.redo_repeat(1)
    }
    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// redo `repeat` time.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    /// commands.undo();
    /// commands.undo();
    /// let v: Vec<_> = commands.redo_repeat(2).collect();
    ///
    /// assert_eq!(v, [Do(&A),Do(&B)]);
    /// ```
    #[must_use = "the returned actions should be realized"]
    pub fn redo_repeat(&mut self, repeat: usize) -> ActionIter<'_, T> {
        let Some(repeat) = repeat.checked_sub(1) else {
            return ActionIter::new();
        };
        let l = self.len();
        match self.commands.last_mut() {
            Some(CommandItem::Undo(i)) => {
                if let Some(ni) = i.checked_sub(repeat.checked_add(1).unwrap()) {
                    let t = l - 2 - *i;
                    *i = ni;
                    ActionIter::do_at_count(&self.commands, &mut self.undo_cache, t, repeat)
                } else {
                    let count = *i;
                    self.commands.pop();
                    ActionIter::do_at_count(&self.commands, &mut self.undo_cache, l - 2, count)
                }
            }
            _ => ActionIter::new(),
        }
    }
    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// undo all commands (to return to the initial state).
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    ///
    /// let v: Vec<_> = commands.undo_all().collect();
    /// assert_eq!(v, [Undo(&B), Undo(&A)]);
    /// ```
    pub fn undo_all(&mut self) -> ActionIter<'_, T> {
        use CommandItem::*;
        let j = match self.last() {
            None => return ActionIter::new(),
            Some(Command(_)) => self.len(),
            Some(Undo(i)) => self.len() - 2 - i,
        };
        self.undo_repeat(j)
    }
    /// Return an iterator over a sequence of actions to be performed by the client application to
    /// redo all undo.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Action,Commands};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    /// commands.undo();
    /// commands.undo();
    ///
    /// let v: Vec<_> = commands.redo_all().collect();
    /// assert_eq!(v, [Do(&A), Do(&B)]);
    /// ```
    #[must_use = "the returned actions should be realized"]
    pub fn redo_all(&mut self) -> ActionIter<'_, T> {
        let l = self.len();
        match self.commands.last_mut() {
            Some(CommandItem::Undo(i)) => {
                let count = *i;
                self.commands.pop();
                ActionIter::do_at_count(&self.commands, &mut self.undo_cache, l - 2, count)
            }
            _ => ActionIter::new(),
        }
    }
    /// Return `true` if the last action is an undo.
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    /// assert!(!commands.is_undoing());
    ///
    /// commands.push(A);
    /// assert!(!commands.is_undoing());
    ///
    /// commands.undo();
    /// assert!(commands.is_undoing());
    ///
    /// commands.push(A);
    /// commands.push(A);
    /// commands.undo();
    /// commands.undo();
    /// assert!(commands.is_undoing());
    /// commands.redo();
    /// assert!(commands.is_undoing());
    /// commands.redo();
    /// assert!(!commands.is_undoing());
    /// ```
    pub fn is_undoing(&self) -> bool {
        matches!(self.commands.last(), Some(CommandItem::Undo(_)))
    }

    /// Check weither there are still command that can be undone
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    /// assert!(!commands.can_undo());
    ///
    /// commands.push(A);
    /// assert!(commands.can_undo());
    ///
    /// commands.undo();
    /// assert!(!commands.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        match self.commands.last() {
            None => false,
            Some(CommandItem::Command(_)) => true,
            Some(CommandItem::Undo(i)) => i + 2 < self.commands.len(),
        }
    }
    /// Check weither there are still command that can be redone
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    /// assert!(!commands.can_redo());
    ///
    /// commands.push(A);
    /// assert!(!commands.can_redo());
    ///
    /// commands.undo();
    /// assert!(commands.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        self.is_undoing()
    }

    /// Return the index of the first realized [command item](CommandItem).
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::*;
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut c = Commands::new();
    ///
    /// c.push(A);
    /// c.push(B);
    ///
    /// let v: Vec<_> = c.iter().collect();
    /// assert_eq!(v, [&CommandItem::Command(A), &CommandItem::Command(B)]);
    ///
    /// assert_eq!(c.current_command_index(), Some(1));
    ///
    /// c.undo();
    ///
    /// let v: Vec<_> = c.iter().collect();
    /// assert_eq!(v, [&CommandItem::Command(A), &CommandItem::Command(B), &CommandItem::Undo(0)]);
    ///
    /// assert_eq!(c.current_command_index(), Some(0));
    /// ```
    pub fn current_command_index(&self) -> Option<usize> {
        let mut it = self.iter_realized();
        it.next()?;
        Some(it.current)
    }

    /// Repeat undo or redo so that the last realiazed command correspond to
    /// the [`CommandItem`] index passed `index`.
    ///
    /// ```
    /// use bevy_text::undo_2::{Action,Commands, CommandItem};
    /// use std::time::{Instant, Duration};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    /// use Command::*;
    /// use Action::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// let t0 = Instant::now();
    /// let t1 = t0 + Duration::from_secs(1);
    /// let t2 = t0 + Duration::from_secs(2);
    /// let t3 = t0 + Duration::from_secs(3);
    /// let t4 = t0 + Duration::from_secs(4);
    /// commands.push((t0,A));
    /// commands.push((t1,B));
    /// commands.undo();
    /// commands.push((t2,C));
    /// commands.push((t3,D));
    /// commands.undo();
    /// commands.undo();
    /// commands.push((t4,E));
    ///
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t4,E),&(t0,A)]);
    ///
    /// let index = commands.iter().position(|item| match item {
    ///         CommandItem::Command(item) => item.0 == t2,
    ///         _ => false
    ///     }).unwrap();
    ///
    /// let actions = commands.undo_or_redo_to_index(index);
    /// let a: Vec<_> = actions.collect();
    /// assert_eq!(a, [Undo(&(t4,E)), Do(&(t2,C))]);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t2,C),&(t0,A)]);
    /// ```
    pub fn undo_or_redo_to_index(&mut self, i: usize) -> ActionIter<'_, T> {
        use CommandItem::*;
        let j = match self.last() {
            None => return ActionIter::new(),
            Some(Command(_)) => self.len(),
            Some(Undo(i)) => self.len() - 2 - i,
        };
        if i >= j {
            self.redo_repeat(i + 1 - j)
        } else {
            self.undo_repeat(j - i - 1)
        }
    }
    /// Clear all the commands.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    ///
    /// commands.clear();
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v.len(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    /// Remove all removable commands that have been added before the
    /// first item fulfilling the predicate.
    ///
    /// A command is removable if it was added before the predicate fulfilling item
    /// and is not covered by any undo.
    ///
    /// Complexity: O(n)
    ///
    /// ```
    /// use bevy_text::undo_2::Commands;
    /// use std::time::{Instant, Duration};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// let t0 = Instant::now();
    /// let t1 = t0 + Duration::from_secs(1);
    /// let t2 = t0 + Duration::from_secs(2);
    /// let t3 = t0 + Duration::from_secs(3);
    /// let t4 = t0 + Duration::from_secs(4);
    /// commands.push((t0,A));
    /// commands.push((t1,B));
    /// commands.push((t2,C));
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// commands.remove_until(|(t, _)| *t > t1);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t4,E),&(t3,D), &(t2,C)]);
    ///
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push((t0,A));
    /// commands.push((t1,B)); //B
    /// commands.push((t2,C));
    /// commands.undo();
    /// commands.undo();// undo covering B
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// commands.remove_until(|(t, _)| *t > t1);
    ///
    /// commands.undo();//remove E
    /// commands.undo();//remove D
    /// commands.undo();//undo the 2 undos
    ///
    /// // B not removed because it is covered by an undo
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t2,C),&(t1,B)]);
    ///
    /// ```
    pub fn remove_until(&mut self, mut stop_pred: impl FnMut(&T) -> bool) {
        if let Some(i) = self.commands.iter().position(move |c| match c {
            CommandItem::Undo(_) => false,
            CommandItem::Command(c) => stop_pred(c),
        }) {
            self.remove_first(i);
        }
    }
    /// Try to keep `count` most recent commands by dropping removable commands.
    ///
    /// A command is removable if it was added before the 'count' last commands
    /// and is not covered by any undo.
    ///
    /// Complexity: O(n)
    ///
    /// ```
    /// use bevy_text::undo_2::Commands;
    /// use std::time::{Instant, Duration};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// let t0 = Instant::now();
    /// let t1 = t0 + Duration::from_secs(1);
    /// let t2 = t0 + Duration::from_secs(2);
    /// let t3 = t0 + Duration::from_secs(3);
    /// let t4 = t0 + Duration::from_secs(4);
    /// commands.push((t0,A));
    /// commands.push((t1,B));
    /// commands.push((t2,C));
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// commands.keep_last(2);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t4,E),&(t3,D)]);
    ///
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push((t0,A));
    /// commands.push((t1,B)); //B
    /// commands.push((t2,C));
    /// commands.undo();
    /// commands.undo();// undo covering B
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// // sequence of undo count for 1
    /// // so try to remove A and B
    /// commands.keep_last(4);
    ///
    /// commands.undo();//remove E
    /// commands.undo();//remove D
    /// commands.undo();//undo the 2 undos
    ///
    /// // B not removed because it is covered by an undo
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t2,C),&(t1,B)]);
    /// ```
    pub fn keep_last(&mut self, count: usize) {
        let i = self.len().saturating_sub(count);
        self.remove_first(i);
    }
    /// Remove `count` or less of the oldest command.
    ///
    /// Less commands may be dropped to ensure that none of the dropped
    /// command is covered by an undo within the recent non dropped commands.
    ///
    /// Complexity: O(n)
    ///
    /// ```
    /// use bevy_text::undo_2::Commands;
    /// use std::time::{Instant, Duration};
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// let t0 = Instant::now();
    /// let t1 = t0 + Duration::from_secs(1);
    /// let t2 = t0 + Duration::from_secs(2);
    /// let t3 = t0 + Duration::from_secs(3);
    /// let t4 = t0 + Duration::from_secs(4);
    /// commands.push((t0,A));
    /// commands.push((t1,B));
    /// commands.push((t2,C));
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// commands.remove_first(3);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t4,E),&(t3,D)]);
    ///
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push((t0,A));
    /// commands.push((t1,B)); //B
    /// commands.push((t2,C));
    /// commands.undo();
    /// commands.undo();// undo covering B
    /// commands.push((t3,D));
    /// commands.push((t4,E));
    ///
    /// // sequence of undo count for 1
    /// // so try to remove A and B
    /// commands.remove_first(2);
    ///
    /// commands.undo();//remove E
    /// commands.undo();//remove D
    /// commands.undo();//undo the 2 undos
    ///
    /// // B not removed because it is covered by an undo
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&(t2,C),&(t1,B)]);
    /// ```
    ///
    /// # Panic
    ///
    /// Panics if `i` is greater than `self.len()`
    pub fn remove_first(&mut self, i: usize) {
        let j = self.remove_i(i, self.commands.len());
        self.commands.drain(..j);
    }
    fn remove_i(&self, mut i: usize, end: usize) -> usize {
        let i0 = i;
        for j in i0..end {
            if let CommandItem::Undo(count) = self.commands[j]
                && j - count - 1 < i
            {
                i = self.remove_i(j - count - 1, i);
            }
        }
        i
    }
    /// Iterate the sequence of [*realized commands*](Commands#method.iter_realized) from the
    /// newest to the oldest.
    ///
    /// *Realized commands* are commands that are not undone. For example assuming
    /// the folowing sequence of commands:
    ///
    /// | Command | State |
    /// |---------|-------|
    /// | Init    |       |
    /// | Do A    | A     |
    /// | Do B    | A, B  |
    /// | Undo    | A     |
    /// | Do C    | A, C  |
    ///
    ///  The iterator would iterator over the sequence [C, A].
    ///
    /// ```
    /// use bevy_text::undo_2::Commands;
    ///
    /// #[derive(Debug, Eq, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    /// use Command::*;
    ///
    /// let mut commands = Commands::new();
    ///
    /// commands.push(A);
    /// commands.push(B);
    /// commands.push(C);
    /// commands.undo();
    /// commands.undo();
    /// commands.push(D);
    /// commands.push(E);
    ///
    /// let v: Vec<_> = commands.iter_realized().collect();
    /// assert_eq!(v, [&E,&D, &A]);
    /// ```
    pub fn iter_realized(&self) -> IterRealized<'_, T> {
        IterRealized {
            commands: &self.commands,
            current: self.commands.len(),
        }
    }
    /// Merge a sequence of [*realized commands*](Commands#method.iter_realized) into a single new
    /// command or remove the sequence.
    ///
    /// The parameter `f` takes as an input a [`IterRealized`], and returns a
    /// [`std::ops::ControlFlow<Option<Merge>, Option<Merge>>`](std::ops::ControlFlow). If the
    /// returned value contain a `Some(merge)`[Merge], the action specified by `merge` is then
    /// inserted in place.
    ///
    /// The function is excuted recursively while it returns a `ControlFlow::Continue(_)` with a
    /// [realized iterator](Commands#method.iter_realized) that is advanced by 1 if no merge
    /// command is returned, or set to the element previous to the last merge command.
    ///
    /// The execution stops when the functions either returned `ControlFlow::Break` or after the
    /// last element in the iteration.
    ///
    /// *Remember*: the element are iterated from the newest to the oldest (in reverse order).
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem, Merge, IterRealized};
    /// use std::ops::ControlFlow;
    ///
    /// #[derive(Eq, PartialEq, Debug)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     AB,
    /// }
    ///
    /// use Command::*;
    ///
    /// fn is_ab<'a>(mut it: IterRealized<'a, Command>) -> (bool, IterRealized<'a, Command>) {
    ///     let cond = it.next() == Some(&B) && it.next() == Some(&A);
    ///     (cond, it)
    /// }
    ///
    ///
    /// let mut commands = Commands::new();
    /// commands.push(A);
    /// commands.push(B);
    ///
    /// commands.merge(|start| {
    ///     if let (true, end) = is_ab(start.clone()) {
    ///         ControlFlow::Continue(Some(Merge {
    ///             start,
    ///             end,
    ///             command: Some(AB),
    ///         }))
    ///     } else {
    ///         ControlFlow::Continue(None)
    ///     }
    /// });
    ///
    /// assert_eq!(&*commands, &[AB.into()]);
    /// ```
    pub fn merge<F>(&mut self, mut f: F)
    where
        for<'a> F:
            FnMut(IterRealized<'a, T>) -> ControlFlow<Option<Merge<'a, T>>, Option<Merge<'a, T>>>,
    {
        use ControlFlow::*;
        self.splice(|it| match f(it) {
            Continue(c) => Continue(c.map(Into::into)),
            Break(c) => Break(c.map(Into::into)),
        });
    }

    /// Replace a sequence of command by an other. This is a generalization of
    /// [`Commands::merge`](Commands#method.merge)
    ///
    /// The parameter `f` takes as an input a [`IterRealized`], and returns a
    /// [`std::ops::ControlFlow<Option<Splice>, Option<Splice>>`](std::ops::ControlFlow). If the returned value
    /// contain a `Some(splice)`[Splice], the actions specified by `splice` are then inserted in
    /// place.
    ///
    /// The function is excuted recursively while it returns a `ControlFlow::Continue(_)` with a
    /// [realized iterator](Commands#method.iter_realized) that is advanced by 1 if no merge
    /// command is returned, or set to the element previous to the last merge command.
    ///
    /// The execution stops when the functions either returned `ControlFlow::Break` or after the
    /// last element in the iteration.
    ///
    /// *Remember*: the element are iterated from the newest to the oldest (in reverse order).
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_text::undo_2::{Commands, CommandItem, Splice, IterRealized};
    /// use std::ops::ControlFlow;
    ///
    /// // we suppose that A, B, C is equivalent to D,E
    /// #[derive(Eq, PartialEq, Debug)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    /// }
    ///
    /// use Command::*;
    ///
    /// fn is_abc<'a>(mut it: IterRealized<'a, Command>) -> (bool, IterRealized<'a, Command>) {
    ///     let cond = it.next() == Some(&C) && it.next() == Some(&B) && it.next() == Some(&A);
    ///     (cond, it)
    /// }
    ///
    ///
    /// let mut commands = Commands::new();
    /// commands.push(A);
    /// commands.push(B);
    /// commands.push(C);
    ///
    /// commands.splice(|start| {
    ///     if let (true, end) = is_abc(start.clone()) {
    ///         ControlFlow::Continue(Some(Splice {
    ///             start,
    ///             end,
    ///             commands: [D,E],
    ///         }))
    ///     } else {
    ///         ControlFlow::Continue(None)
    ///     }
    /// });
    ///
    /// assert_eq!(&*commands, &[D.into(), E.into()]);
    /// ```
    pub fn splice<F, I>(&mut self, mut f: F)
    where
        F: for<'a> FnMut(
            IterRealized<'a, T>,
        )
            -> ControlFlow<Option<Splice<'a, T, I>>, Option<Splice<'a, T, I>>>,
        I: IntoIterator<Item = T>,
    {
        use ControlFlow::*;
        let mut start = self.commands.len();
        while start != 0 {
            let it = IterRealized {
                commands: &self.commands,
                current: start,
            };
            match f(it) {
                Continue(Some(m)) => {
                    let rev_start = m.start.current;
                    let rev_end = m.end.current;
                    let commands = m.commands;
                    self.do_splice(rev_start, rev_end, commands);
                    start = rev_end;
                }
                Break(Some(m)) => {
                    let rev_start = m.start.current;
                    let rev_end = m.end.current;
                    let commands = m.commands;
                    self.do_splice(rev_start, rev_end, commands);
                    break;
                }
                Break(None) => break,
                Continue(None) => start -= 1,
            }
        }
    }
    fn do_splice<I>(&mut self, rev_start: usize, rev_end: usize, commands: I)
    where
        I: IntoIterator<Item = T>,
    {
        let end_i = rev_start;
        let start_i = rev_end;
        self.commands
            .splice(start_i..end_i, commands.into_iter().map(Into::into));
    }

    /// Clean up the history of all the undone commands.
    ///
    /// After this call the sequence of command will not contain
    /// any `CommandItem::Undo`
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{CommandItem, Commands};
    ///
    /// #[derive(Debug, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    /// }
    /// use Command::*;
    /// let mut c = Commands::default();
    ///
    /// c.push(A);
    /// c.push(B);
    /// c.undo();
    /// c.push(C);
    /// assert_eq!(*c, [A.into(), B.into(), CommandItem::Undo(0), C.into()]);
    ///
    /// c.remove_all_undone();
    /// assert_eq!(*c, [A.into(), C.into()]);
    /// ```
    pub fn remove_all_undone(&mut self) {
        self.remove_undone(|i| i);
    }
    /// Clean up the history of all undone commands before a given
    /// [realized iterator](Commands#method.iter_realized).
    ///
    /// # Example
    /// ```
    /// use bevy_text::undo_2::{Commands,CommandItem};
    ///
    /// #[derive(Debug, PartialEq)]
    /// enum Command {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    /// }
    /// use Command::*;
    /// let mut c = Commands::default();
    ///
    /// c.push(A);
    /// c.push(B);
    /// c.undo();
    /// c.push(C);
    /// c.push(C);
    /// c.undo();
    /// c.push(D);
    /// assert_eq!(
    ///     *c,
    ///     [
    ///         A.into(),
    ///         B.into(),
    ///         CommandItem::Undo(0),
    ///         C.into(),
    ///         C.into(),
    ///         CommandItem::Undo(0),
    ///         D.into()
    ///     ]
    /// );
    ///
    /// let v: Vec<_> = c.iter_realized().collect();
    /// assert_eq!(*v, [&D, &C, &A]);
    ///
    /// c.remove_undone(|mut it| {
    ///     it.nth(1);
    ///     it
    /// });
    /// assert_eq!(
    ///     *c,
    ///     [A.into(), C.into(), C.into(), CommandItem::Undo(0), D.into()]
    /// );
    ///
    /// // This operation does not change the sequence of realized commands:
    /// let v: Vec<_> = c.iter_realized().collect();
    /// assert_eq!(*v, [&D, &C, &A]);
    /// ```
    pub fn remove_undone<F>(&mut self, from: F)
    where
        F: for<'a> FnOnce(IterRealized<'a, T>) -> IterRealized<'a, T>,
    {
        use CachedAction::*;
        let from = from(self.iter_realized());

        let mut it = IterRealized {
            commands: &self.commands,
            ..from
        };

        self.undo_cache.clear();

        let start = it.current;
        while let Some(_) = it.next() {
            self.undo_cache.push(IndexedAction {
                action: Do,
                index: it.current,
            });
        }

        let mut i = 0;
        let mut shift = 0;
        for u in self.undo_cache.iter().rev() {
            let j = u.index;
            self.commands.drain(i - shift..j - shift);
            shift += j - i;
            i = j + 1;
        }
        self.commands.drain(i - shift..start - shift);
    }
}

impl<T: Clone> Clone for Commands<T> {
    fn clone(&self) -> Self {
        Self {
            commands: self.commands.clone(),
            undo_cache: vec![],
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.commands.clone_from(&source.commands);
    }
}

impl<T> Deref for Commands<T> {
    type Target = [CommandItem<T>];
    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}
impl<T> AsRef<[CommandItem<T>]> for Commands<T> {
    fn as_ref(&self) -> &[CommandItem<T>] {
        self
    }
}
impl<T> Borrow<[CommandItem<T>]> for Commands<T> {
    fn borrow(&self) -> &[CommandItem<T>] {
        self
    }
}
impl<T> Extend<T> for Commands<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.commands.extend(iter.into_iter().map(Into::into));
    }
}
impl<T> FromIterator<T> for Commands<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            commands: iter.into_iter().map(Into::into).collect(),
            undo_cache: vec![],
        }
    }
}
impl<'a, T> IntoIterator for &'a Commands<T> {
    type Item = &'a CommandItem<T>;
    type IntoIter = slice::Iter<'a, CommandItem<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.commands.iter()
    }
}
impl<T> IntoIterator for Commands<T> {
    type Item = CommandItem<T>;
    type IntoIter = vec::IntoIter<CommandItem<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.commands.into_iter()
    }
}
impl<T, I> Index<I> for Commands<T>
where
    I: SliceIndex<[CommandItem<T>]>,
{
    type Output = <I as SliceIndex<[CommandItem<T>]>>::Output;
    fn index(&self, index: I) -> &Self::Output {
        self.commands.index(index)
    }
}

impl<T> From<T> for CommandItem<T> {
    fn from(value: T) -> Self {
        CommandItem::Command(value)
    }
}

impl<'a, T> From<Merge<'a, T>> for Splice<'a, T, option::IntoIter<T>> {
    fn from(m: Merge<'a, T>) -> Self {
        Splice {
            start: m.start,
            end: m.end,
            commands: m.command.into_iter(),
        }
    }
}

impl<T> IterRealized<'_, T> {
    /// Returned the index of the command refered by the previous non `None` result of call to
    /// `next`.
    ///
    /// This same command is accessible by indexing [Commands] at this returned index.
    ///
    /// This index can be used to set the first realized command with
    /// [`Commands::undo_or_redo_to_index`](Commands#method.undo_or_redo_to_index).
    pub fn index(&self) -> usize {
        self.current
    }
}

impl<'a, T> Iterator for IterRealized<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        use CommandItem::*;
        loop {
            self.current = self.current.checked_sub(1)?;
            match self.commands[self.current] {
                Command(ref c) => break Some(c),
                Undo(i) => self.current -= i + 1,
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.current))
    }
}
impl<T> FusedIterator for IterRealized<'_, T> {}

impl<'a, T> Iterator for ActionIter<'a, T> {
    type Item = Action<&'a T>;
    fn next(&mut self) -> Option<Self::Item> {
        use CachedAction::*;
        loop {
            let a = self.to_do.next()?;
            match a {
                IndexedAction { action: Do, index } => {
                    break if let CommandItem::Command(v) = &self.commands[*index] {
                        Some(Action::Do(v))
                    } else {
                        unreachable!()
                    }
                }
                IndexedAction {
                    action: Undo,
                    index,
                } => {
                    break if let CommandItem::Command(v) = &self.commands[*index] {
                        Some(Action::Undo(v))
                    } else {
                        unreachable!()
                    }
                }
                IndexedAction { action: Skip, .. } => (),
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.to_do.size_hint()
    }
}
impl<T> FusedIterator for ActionIter<'_, T> {}
impl<T> ExactSizeIterator for ActionIter<'_, T> {}

impl IndexedAction {
    fn is_reverse_of(&self, other: &Self) -> bool {
        use CachedAction::*;
        self.index == other.index
            && (self.action == Do && other.action == Undo
                || self.action == Undo && other.action == Do)
    }
}

impl<'a, T> ActionIter<'a, T> {
    fn new() -> Self {
        Self {
            commands: &[],
            to_do: [].iter(),
        }
    }
    fn undo_at_count(
        commands: &'a [CommandItem<T>],
        cache: &'a mut Vec<IndexedAction>,
        i: usize,
        count: usize,
    ) -> Self {
        cache.clear();
        cache_undo_indexes(commands, i + 1 + count, i, cache);
        do_simplify(cache);
        Self {
            commands,
            to_do: cache.iter(),
        }
    }
    fn do_at_count(
        commands: &'a [CommandItem<T>],
        cache: &'a mut Vec<IndexedAction>,
        i: usize,
        count: usize,
    ) -> Self {
        cache.clear();
        cache_do_indexes(commands, i - count, i + 1, cache);
        do_simplify(cache);
        Self {
            commands,
            to_do: cache.iter(),
        }
    }
}
fn cache_undo_indexes<T>(
    commands: &[CommandItem<T>],
    undo_from: usize,
    undo_to: usize,
    to_do: &mut Vec<IndexedAction>,
) {
    use CachedAction::*;
    for i in (undo_to..undo_from).rev() {
        match commands[i] {
            CommandItem::Command(_) => to_do.push(IndexedAction {
                action: Undo,
                index: i,
            }),
            CommandItem::Undo(count) => cache_do_indexes(commands, i - (count + 1), i, to_do),
        }
    }
}
fn cache_do_indexes<T>(
    commands: &[CommandItem<T>],
    do_from: usize,
    do_to: usize,
    to_do: &mut Vec<IndexedAction>,
) {
    use CachedAction::*;
    for i in do_from..do_to {
        match commands[i] {
            CommandItem::Command(_) => to_do.push(IndexedAction {
                action: Do,
                index: i,
            }),
            CommandItem::Undo(count) => cache_undo_indexes(commands, i, i - (count + 1), to_do),
        }
    }
}
fn do_simplify(to_do: &mut [IndexedAction]) {
    use CachedAction::*;
    if to_do.len() < 2 {
        return;
    }
    let mut analyzed = to_do.len() - 1;
    let mut cursor = to_do.len() - 1;
    while analyzed > 0 {
        analyzed -= 1;
        let action = &to_do[analyzed];
        let l = to_do.len();
        if cursor < to_do.len() {
            if to_do[cursor].is_reverse_of(action) {
                cursor += 1;
                while cursor < l && to_do[cursor].action == Skip {
                    cursor += 1;
                }
            } else {
                to_do[analyzed + 1..cursor]
                    .iter_mut()
                    .for_each(|a| a.action = Skip);
                if cursor == analyzed + 1 {
                    cursor = analyzed;
                } else {
                    cursor = analyzed + 1;
                    analyzed += 1;
                }
            }
        } else {
            to_do[analyzed + 1..]
                .iter_mut()
                .for_each(|a| a.action = Skip);
            cursor = analyzed;
        }
    }
    to_do[..cursor].iter_mut().for_each(|a| a.action = Skip);
}

#[cfg(test)]
mod test {
    use super::CachedAction::*;
    use super::IndexedAction;
    #[test]
    fn simplify() {
        use super::do_simplify;
        fn simplify(mut to_do: Vec<IndexedAction>) -> Vec<IndexedAction> {
            do_simplify(&mut to_do);
            to_do.iter().filter(|c| c.action != Skip).copied().collect()
        }
        fn _do(i: usize) -> IndexedAction {
            IndexedAction {
                action: Do,
                index: i,
            }
        }
        fn undo(i: usize) -> IndexedAction {
            IndexedAction {
                action: Undo,
                index: i,
            }
        }
        {
            let v = vec![];
            assert_eq!(simplify(v), vec![]);
        }
        {
            let v = vec![_do(1)];
            assert_eq!(simplify(v), vec![_do(1)]);
        }
        {
            let v = vec![undo(1)];
            assert_eq!(simplify(v), vec![undo(1)]);
        }
        {
            let v = vec![_do(1), undo(1)];
            assert_eq!(simplify(v), vec![]);
        }
        {
            let v = vec![_do(0), _do(1), undo(1)];
            assert_eq!(simplify(v), vec![_do(0)]);
        }
        {
            let v = vec![_do(1), undo(1), _do(2)];
            assert_eq!(simplify(v), vec![_do(2)]);
        }
        {
            let v = vec![_do(0), _do(1), undo(1), _do(2)];
            assert_eq!(simplify(v), vec![_do(0), _do(2)]);
        }
        {
            let v = vec![_do(1), _do(2), undo(2), undo(1)];
            assert_eq!(simplify(v), vec![]);
        }
        {
            let v = vec![_do(0), _do(1), _do(2), undo(2), undo(1)];
            assert_eq!(simplify(v), vec![_do(0)]);
        }
        {
            let v = vec![_do(1), _do(2), undo(2), undo(1), _do(3)];
            assert_eq!(simplify(v), vec![_do(3)]);
        }
        {
            let v = vec![_do(0), _do(1), _do(2), undo(2), undo(1), _do(3)];
            assert_eq!(simplify(v), vec![_do(0), _do(3)]);
        }
        {
            let v = vec![_do(0), _do(1), _do(2), undo(2), undo(1), undo(0)];
            assert_eq!(simplify(v), vec![]);
        }
        {
            let v = vec![
                _do(0),
                _do(1),
                _do(2),
                undo(2),
                undo(1),
                _do(10),
                undo(10),
                undo(0),
            ];
            assert_eq!(simplify(v), vec![]);
        }
    }
}
