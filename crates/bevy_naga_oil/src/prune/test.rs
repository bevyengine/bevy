#![allow(unused_imports)]
use crate::prune::{PartReq, Pruner};

use super::*;
use naga::{
    back::wgsl::WriterFlags,
    valid::{Capabilities, ValidationFlags},
};

#[test]
fn it_works() {
    let shader_src = include_str!("tests/test.wgsl");
    let shader = naga::front::wgsl::parse_str(shader_src).unwrap();
    println!("{:#?}", shader);

    let info = naga::valid::Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&shader)
        .unwrap();
    let text = naga::back::wgsl::write_string(&shader, &info, WriterFlags::EXPLICIT_TYPES).unwrap();
    println!("\n\nbase wgsl:\n{}", text);

    let mut modreq = Pruner::new(&shader);
    let func = shader
        .functions
        .fetch_if(|f| f.name == Some("test".to_string()))
        .unwrap();
    let input_req = modreq.add_function(
        func,
        Default::default(),
        Some(PartReq::Part([(0, PartReq::All)].into())),
    );

    println!("\n\ninput_req:\n{:#?}", input_req);
    println!("\n\nmodreq:\n{:#?}", modreq);

    let rewritten_shader = modreq.rewrite();

    println!("\n\nrewritten_shader:\n{:#?}", rewritten_shader);

    let info = naga::valid::Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&rewritten_shader)
        .unwrap();
    let text =
        naga::back::wgsl::write_string(&rewritten_shader, &info, WriterFlags::EXPLICIT_TYPES)
            .unwrap();
    println!("\n\nwgsl:\n{}", text);
}

#[test]
fn frag_reduced() {
    let shader_src = include_str!("tests/frag_reduced.wgsl");
    let shader = naga::front::wgsl::parse_str(shader_src).unwrap();

    let mut pruner = Pruner::new(&shader);
    let context = pruner.add_entrypoint(
        shader.entry_points.get(0).unwrap(),
        Default::default(),
        None,
    );
    println!("{:?}", context);
}

#[test]
fn frag_reduced_2() {
    let shader_src = include_str!("tests/frag_reduced_2.wgsl");
    let shader = naga::front::wgsl::parse_str(shader_src).unwrap();

    let mut pruner = Pruner::new(&shader);
    let context = pruner.add_entrypoint(
        shader.entry_points.get(0).unwrap(),
        Default::default(),
        None,
    );
    println!("{:?}", context);
}

#[test]
fn pbr_reduced() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    let _trace = tracing::subscriber::set_default(subscriber);

    let shader_src = include_str!("tests/pbr_reduced.wgsl");
    let shader = naga::front::wgsl::parse_str(shader_src).unwrap();

    println!("{:#?}", shader);

    let mut pruner = Pruner::new(&shader);
    let context = pruner.add_entrypoint(
        shader.entry_points.get(0).unwrap(),
        Default::default(),
        None,
    );
    println!("{:?}", context);
    context.globals_for_module(&shader);

    let rewrite = pruner.rewrite();
    let info = naga::valid::Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&rewrite)
        .unwrap();
    let text =
        naga::back::wgsl::write_string(&rewrite, &info, WriterFlags::EXPLICIT_TYPES).unwrap();
    println!("\n\nwgsl:\n{}", text);
}

#[test]
fn pbr_fn() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    let _trace = tracing::subscriber::set_default(subscriber);

    let shader_src = include_str!("tests/pbr_fn.wgsl");
    let shader = naga::front::wgsl::parse_str(shader_src).unwrap();

    println!("{:#?}", shader);

    let mut pruner = Pruner::new(&shader);
    let (req, context) = pruner.add_function(
        shader.functions.iter().next().unwrap().0,
        Default::default(),
        None,
    );
    println!("{}: {:?}", req, context);
    context.globals_for_module(&shader);

    let rewrite = pruner.rewrite();
    let info = naga::valid::Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&rewrite)
        .unwrap();
    let text =
        naga::back::wgsl::write_string(&rewrite, &info, WriterFlags::EXPLICIT_TYPES).unwrap();
    println!("\n\nwgsl:\n{}", text);
}
