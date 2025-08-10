#![allow(unused, reason = "tests")]

use crate::undo_2::*;
use core::ops::ControlFlow;

#[derive(PartialEq, Debug)]
enum Command {
    A,
    B,
    C,
    D,
    E,
}
use Command::*;

fn is_abc(mut it: IterRealized<'_, Command>) -> (bool, IterRealized<'_, Command>) {
    let cond = it.next() == Some(&C) && it.next() == Some(&B) && it.next() == Some(&A);
    (cond, it)
}

fn do_splice(c: &mut Commands<Command>) {
    c.splice(|start| {
        if let (true, end) = is_abc(start.clone()) {
            ControlFlow::Continue(Some(Splice {
                start,
                end,
                commands: [D, E],
            }))
        } else {
            ControlFlow::Continue(None)
        }
    });
}

#[test]
fn splice_without_undos() {
    {
        let mut c = Commands::default();

        do_splice(&mut c);
        assert_eq!(*c, []);

        c.push(A);

        do_splice(&mut c);
        assert_eq!(*c, [A.into()]);

        c.push(B);

        do_splice(&mut c);
        assert_eq!(*c, [A.into(), B.into()]);

        c.push(C);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into()]);

        c.push(A);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), A.into()]);

        c.push(B);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), A.into(), B.into()]);

        c.push(C);
    }
    {
        let mut c = Commands::default();

        do_splice(&mut c);
        assert_eq!(*c, []);

        c.push(A);

        do_splice(&mut c);
        assert_eq!(*c, [A.into()]);

        c.push(B);

        do_splice(&mut c);
        assert_eq!(*c, [A.into(), B.into()]);

        c.push(C);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into()]);

        c.push(D);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), D.into()]);

        c.push(A);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), D.into(), A.into()]);

        c.push(B);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), D.into(), A.into(), B.into()]);

        c.push(C);
        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), D.into(), D.into(), E.into()]);
    }
    {
        let mut c = Commands::default();

        do_splice(&mut c);
        assert_eq!(*c, []);

        c.push(A);
        c.push(B);
        c.push(C);
        c.push(D);
        c.push(A);
        c.push(B);
        c.push(C);

        do_splice(&mut c);
        assert_eq!(*c, [D.into(), E.into(), D.into(), D.into(), E.into()]);
    }
}

#[test]
fn splice_with_undos() {
    {
        let mut c = Commands::default();

        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert!(v.is_empty());

        c.push(A);
        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&A]);

        c.push(D);
        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&D, &A]);

        c.undo();
        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&A]);

        c.push(B);
        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&B, &A]);

        c.push(C);
        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&E, &D]);
        assert_eq!(*c, [D.into(), E.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.push(C);
        c.undo();
        c.undo();
        c.push(D);
        c.push(E);
        c.undo();
        c.undo();
        c.undo();
        c.push(E);

        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&E, &E, &D]);
        assert_eq!(*c, [D.into(), E.into(), E.into()]);
    }
    {
        let mut c = Commands::default();

        c.push(A);
        c.push(B);
        c.push(C);
        c.push(C);
        c.undo();

        do_splice(&mut c);
        let v: Vec<_> = c.iter_realized().collect();
        assert_eq!(*v, [&E, &D]);
        assert_eq!(*c, [D.into(), E.into()]);
    }
}
