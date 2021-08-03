#![allow(dead_code)]
use bevy_ecs::prelude::*;

fn main() {}

struct A;
struct B;
struct C;

fn test_query1(_query: Query<&A, With<A>>) {
    test_query1.system();
}
fn test_query2(_query: Query<(&A, &B), With<A>>) {
    test_query2.system();
}
fn test_query3(_query: Query<(&A, &B), With<B>>) {
    test_query3.system();
}
fn test_query4(_query: Query<(&A, &B), (With<A>, With<B>)>) {
    test_query4.system();
}
fn test_query5(_query: Query<(&A, &B), (With<A>, With<C>)>) {
    test_query5.system();
}
fn test_query6(_query: Query<&A, (With<A>, With<B>)>) {
    test_query6.system();
}

fn test_query7(_query: Query<&mut A, With<A>>) {
    test_query7.system();
}
fn test_query8(_query: Query<(&mut A, &B), With<A>>) {
    test_query8.system();
}
fn test_query9(_query: Query<(&mut A, &B), With<B>>) {
    test_query9.system();
}
fn test_query10(_query: Query<(&mut A, &B), (With<A>, With<B>)>) {
    test_query10.system();
}
fn test_query11(_query: Query<(&mut A, &B), (With<A>, With<C>)>) {
    test_query11.system();
}
fn test_query12(_query: Query<&mut A, (With<A>, With<B>)>) {
    test_query12.system();
}

fn test_query13(_query: Query<(), (Added<A>, With<A>)>) {
    test_query13.system();
}
fn test_query14(_query: Query<(), (Changed<A>, With<A>)>) {
    test_query14.system();
}

fn test_query15(_query: Query<&A, Or<(With<A>, With<B>)>>) {
    test_query15.system();
}
fn test_query16(_query: Query<&B, Or<(With<A>, With<B>)>>) {
    test_query16.system();
}

fn test_query17(_query: Query<&mut A, Or<(With<A>, With<B>)>>) {
    test_query17.system();
}
fn test_query18(_query: Query<&mut B, Or<(With<A>, With<B>)>>) {
    test_query18.system();
}

fn test_query19(_query: Query<(), Or<(Added<A>, With<A>)>>) {
    test_query19.system();
}
fn test_query20(_query: Query<(), Or<(Changed<A>, With<A>)>>) {
    test_query20.system();
}

fn test_query21(_query: Query<&A, (Or<(With<A>, With<B>)>, With<C>)>) {
    test_query21.system();
}
fn test_query22(_query: Query<&mut A, (Or<(With<A>, With<B>)>, With<C>)>) {
    test_query22.system();
}
