#![allow(dead_code)]
use bevy_ecs::prelude::*;

fn main() {}

struct A;
struct B;
struct C;

fn test_query1(_query: Query<&A, With<A>>) {}
fn test_query2(_query: Query<(&A, &B), With<A>>) {}
fn test_query3(_query: Query<(&A, &B), With<B>>) {}
fn test_query4(_query: Query<(&A, &B), (With<A>, With<B>)>) {}
fn test_query5(_query: Query<(&A, &B), (With<A>, With<C>)>) {}
fn test_query6(_query: Query<&A, (With<A>, With<B>)>) {}

fn test_query7(_query: Query<&mut A, With<A>>) {}
fn test_query8(_query: Query<(&mut A, &B), With<A>>) {}
fn test_query9(_query: Query<(&mut A, &B), With<B>>) {}
fn test_query10(_query: Query<(&mut A, &B), (With<A>, With<B>)>) {}
fn test_query11(_query: Query<(&mut A, &B), (With<A>, With<C>)>) {}
fn test_query12(_query: Query<&mut A, (With<A>, With<B>)>) {}

fn test_query13(_query: Query<(), (Added<A>, With<A>)>) {}
fn test_query14(_query: Query<(), (Changed<A>, With<A>)>) {}

fn test_query15(_query: Query<&A, Or<(With<A>, With<B>)>>) {}
fn test_query16(_query: Query<&B, Or<(With<A>, With<B>)>>) {}

fn test_query17(_query: Query<&mut A, Or<(With<A>, With<B>)>>) {}
fn test_query18(_query: Query<&mut B, Or<(With<A>, With<B>)>>) {}

fn test_query19(_query: Query<(), Or<(Added<A>, With<A>)>>) {}
fn test_query20(_query: Query<(), Or<(Changed<A>, With<A>)>>) {}

fn test_query21(_query: Query<&A, (Or<(With<A>, With<B>)>, With<C>)>) {}
fn test_query22(_query: Query<&mut A, (Or<(With<A>, With<B>)>, With<C>)>) {}
