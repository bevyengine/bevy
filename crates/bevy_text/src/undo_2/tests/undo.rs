#![allow(unused)]
use undo_2::{Action, Commands};

#[derive(Debug, Eq, PartialEq)]
enum Command {
    A,
    B,
    C,
    D,
}
use Command::*;

#[test]
fn undo() {
    use Action::*;
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Undo(&B)]);

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Undo(&A)]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.undo();
        c.push(C);

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Undo(&C)]);

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Do(&A), Do(&B)]);
    }
    {
        let mut c = Commands::default();

        c.push(A); // A
        c.push(B); // A B
        c.undo(); //  A
        c.undo(); //
        c.push(C); // C
        c.undo(); //
        c.undo(); //  A B
        c.push(D); // A B D

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Undo(&D)]);

        let v: Vec<_> = c.undo().collect();
        assert_eq!(v, [Undo(&B), Undo(&A), Do(&C)]);
    }
}
