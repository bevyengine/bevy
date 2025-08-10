use core::mem::{discriminant, Discriminant};

use crate::undo_2::{IndepStateKey, SetTransAction};

#[derive(Copy, Clone, Debug)]
struct State {
    color: f64,
    length: f64,
}
impl State {
    fn new() -> Self {
        INIT_STATE
    }
    fn apply_set(&mut self, c: &SetCommands) {
        match c {
            SetCommands::Color(v) => self.color = *v,
            SetCommands::Length(v) => self.length = *v,
        };
    }
    fn execute_action(&mut self, c: SetTransAction<SetCommands, TransitionCommand>) {
        match c {
            SetTransAction::Do(_) | SetTransAction::Undo(_) => {}
            SetTransAction::Set(c) => self.apply_set(c),
            SetTransAction::SetToInitial(d) => self.apply_set(SetCommands::new_initial(d)),
        }
    }
}
static INIT_STATE: State = State {
    color: 0.,
    length: 0.,
};

#[derive(Debug, Copy, Clone, PartialEq)]
enum SetCommands {
    Color(f64),
    Length(f64),
}
static INIT_COLOR: SetCommands = SetCommands::Color(INIT_STATE.color);
static INIT_LENGTH: SetCommands = SetCommands::Length(INIT_STATE.length);

impl SetCommands {
    fn new_initial(d: Discriminant<Self>) -> &'static Self {
        if d == discriminant(&INIT_COLOR) {
            &INIT_COLOR
        } else if d == discriminant(&INIT_LENGTH) {
            &INIT_LENGTH
        } else {
            unreachable!("SetCommands::new_initial is not exhaustive: please, adds lacking initial value to this method")
        }
    }
}

impl IndepStateKey for SetCommands {
    type KeyType = Discriminant<Self>;

    fn key(&self) -> Self::KeyType {
        discriminant(self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TransitionCommand {
    A,
    B,
}

#[test]
fn set_and_transition() {
    use crate::undo_2::{Commands, SetOrTransition::*};
    let mut commands = Commands::new();
    let mut state = State::new();

    let c = SetCommands::Color(1.);
    state.apply_set(&c);
    commands.push(Set(c));

    commands.push(Transition(TransitionCommand::A));
    commands.push(Transition(TransitionCommand::B));

    let c = SetCommands::Length(10.);
    state.apply_set(&c);
    commands.push(Set(c));

    let c = SetCommands::Color(2.);
    state.apply_set(&c);
    commands.push(Set(c));

    commands.apply_undo(|c| {
        assert_eq!(c, SetTransAction::Set(&SetCommands::Color(1.)));
        state.execute_action(c);
    });
    assert_eq!(state.color, 1.);
    assert_eq!(state.length, 10.);
    commands.apply_redo(|c| {
        assert_eq!(c, SetTransAction::Set(&SetCommands::Color(2.)));
        state.execute_action(c);
    });
    assert_eq!(state.color, 2.);
    assert_eq!(state.length, 10.);
}
