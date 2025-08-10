#![allow(unused)]
use undo_2::{Action, Commands};

#[derive(Debug, Eq, PartialEq)]
enum Command {
    A,
    B,
    C,
    D,
}
use Action::*;
use Command::*;

#[test]
fn redo() {
    {
        let mut commands = Commands::new();

        commands.push(A); // A
        commands.push(B); // A B
        commands.undo(); // A
        commands.undo(); //
        commands.push(C); // C
        commands.undo(); //
        commands.undo(); // A B

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Undo(&B), Undo(&A)]);

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Do(&C)]);
    }
    {
        let mut commands = Commands::new();

        commands.push(A); // A
        commands.push(B); // A B
        commands.undo(); // A
        commands.undo(); //
                         //
        commands.push(C); // C
                          //
        commands.undo(); //
        commands.undo(); // A B
        commands.undo(); // A
        commands.undo(); //

        commands.push(D); // D

        commands.undo(); //
        commands.undo(); // C
        commands.undo(); //
        commands.undo(); // A B

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Undo(&B), Undo(&A)]);

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Do(&C)]);

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Undo(&C)]);

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, [Do(&D)]);

        let v: Vec<_> = commands.redo().collect();
        assert_eq!(v, []);
    }
}

#[test]
fn redo_all() {
    let mut commands = Commands::new();

    commands.push(A);
    commands.push(B);
    commands.undo();
    commands.undo();

    let v: Vec<_> = commands.redo_all().collect();
    assert_eq!(v, [Do(&A), Do(&Command::B)]);
}
