#![allow(unused, reason = "tests")]
use crate::undo_2::{Action, Commands};

#[derive(Debug, Eq, PartialEq)]
enum Command {
    A,
    B,
    C,
}
use Command::*;

#[test]
fn unbuild() {
    use Action::*;
    let mut commands = Commands::new();

    commands.push(A);
    commands.push(B);
    commands.undo();
    commands.push(C);

    let c: Vec<_> = commands.unbuild().collect();
    assert_eq!(c, [Undo(&C)]);

    let c: Vec<_> = commands.unbuild().collect();
    assert_eq!(c, &[Undo(&A)]);

    let c: Vec<_> = commands.unbuild().collect();
    assert!(c.is_empty());

    dbg!(&commands);
    let v: Vec<_> = commands.rebuild().collect();
    assert_eq!(v, &[Do(&A)]);

    let v: Vec<_> = commands.rebuild().collect();
    assert_eq!(v, &[Do(&C)]);
}
