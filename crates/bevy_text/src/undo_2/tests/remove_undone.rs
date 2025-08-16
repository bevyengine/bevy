#![allow(unused, reason = "tests")]
use crate::undo_2::*;

#[test]
fn remove_undone() {
    #[derive(Debug, PartialEq)]
    enum Command {
        A,
        B,
        C,
    }
    use Command::*;
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        assert_eq!(*c, [A.into(), B.into(), CommandItem::Undo(0), C.into()]);

        c.remove_all_undone();
        assert_eq!(*c, [A.into(), C.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.push(C);
        c.undo();

        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&B, &A]);

        c.remove_all_undone();
        assert_eq!(*c, [A.into(), B.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();

        c.remove_all_undone();
        assert_eq!(*c, [A.into(), C.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();
        c.undo();

        c.remove_all_undone();
        assert_eq!(*c, [A.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();
        c.undo();
        c.undo();

        c.remove_all_undone();
        assert_eq!(*c, [A.into(), B.into()]);
    }
}
fn remove_undo_from() {
    #[derive(Debug, PartialEq)]
    enum Command {
        A,
        B,
        C,
        D,
    }
    use Command::*;
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();
        c.push(D);
        assert_eq!(
            *c,
            [
                A.into(),
                B.into(),
                CommandItem::Undo(0),
                C.into(),
                C.into(),
                CommandItem::Undo(0),
                D.into()
            ]
        );

        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&D, &C, &A]);

        c.remove_undone(|mut it| {
            it.nth(1);
            it
        });
        assert_eq!(
            *c,
            [A.into(), C.into(), C.into(), CommandItem::Undo(0), D.into()]
        );

        // This operation does not change the sequence of realized commands:
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&D, &C, &A]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();
        c.push(D);
        assert_eq!(
            *c,
            [
                A.into(),
                B.into(),
                CommandItem::Undo(0),
                C.into(),
                C.into(),
                CommandItem::Undo(0),
                D.into()
            ]
        );

        c.remove_undone(|mut it| {
            it.next();
            it
        });
        assert_eq!(*c, [A.into(), C.into(), D.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.undo();
        c.push(C);
        c.push(C);
        c.undo();
        c.push(D);
        assert_eq!(
            *c,
            [
                A.into(),
                B.into(),
                CommandItem::Undo(0),
                C.into(),
                C.into(),
                CommandItem::Undo(0),
                D.into()
            ]
        );

        c.remove_undone(|mut it| {
            it.nth(2);
            it
        });
        assert_eq!(*c, []);
    }
}
