#![allow(unused)]

use undo_2::*;

#[derive(PartialEq, Debug)]
enum Command {
    A,
    B,
    C,
    D,
    E,
}
use Command::*;

#[test]
fn iter_realized() {
    let mut c = Commands::default();
    c.push(A);
    c.push(B);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&A]);

    c.push(C);
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&C, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&B, &A]);

    c.push(D);
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&D, &B, &A]);

    c.push(E);
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&E, &D, &B, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&D, &B, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&B, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&C, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(*v, [&A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(v, [&B, &A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(v, [&A]);

    c.undo();
    let v: Vec<_> = c.iter_realized().collect();
    assert_eq!(v, Vec::<&Command>::new());
}
