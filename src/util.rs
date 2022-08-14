// polyfill clones

use naga::{Arena, Constant, EntryPoint, Function, Module, Span, UniqueArena};

// these do not remap handles, only use if the arenas are not modified
pub fn copy_type(t: &naga::Type) -> naga::Type {
    naga::Type {
        name: t.name.clone(),
        inner: match &t.inner {
            naga::TypeInner::Scalar { kind, width } => naga::TypeInner::Scalar {
                kind: *kind,
                width: *width,
            },
            naga::TypeInner::Vector { size, kind, width } => naga::TypeInner::Vector {
                size: *size,
                kind: *kind,
                width: *width,
            },
            naga::TypeInner::Matrix {
                columns,
                rows,
                width,
            } => naga::TypeInner::Matrix {
                columns: *columns,
                rows: *rows,
                width: *width,
            },
            naga::TypeInner::Atomic { kind, width } => naga::TypeInner::Atomic {
                kind: *kind,
                width: *width,
            },
            naga::TypeInner::Pointer { base, space } => naga::TypeInner::Pointer {
                base: *base,
                space: *space,
            },
            naga::TypeInner::ValuePointer {
                size,
                kind,
                width,
                space,
            } => naga::TypeInner::ValuePointer {
                size: *size,
                kind: *kind,
                width: *width,
                space: *space,
            },
            naga::TypeInner::Array { base, size, stride } => naga::TypeInner::Array {
                base: *base,
                size: *size,
                stride: *stride,
            },
            naga::TypeInner::Struct { members, span } => naga::TypeInner::Struct {
                members: members.to_vec(),
                span: *span,
            },
            naga::TypeInner::Image {
                dim,
                arrayed,
                class,
            } => naga::TypeInner::Image {
                dim: *dim,
                arrayed: *arrayed,
                class: *class,
            },
            naga::TypeInner::Sampler { comparison } => naga::TypeInner::Sampler {
                comparison: *comparison,
            },
            naga::TypeInner::BindingArray { base, size } => naga::TypeInner::BindingArray {
                base: *base,
                size: *size,
            },
        },
    }
}

pub fn clone_const(c: &Constant) -> Constant {
    Constant {
        name: c.name.clone(),
        specialization: c.specialization,
        inner: c.inner.clone(),
    }
}

pub fn copy_func(f: &Function) -> Function {
    Function {
        name: f.name.clone(),
        arguments: f.arguments.to_vec(),
        result: f.result.clone(),
        local_variables: clone_arena(&f.local_variables, Clone::clone),
        expressions: clone_arena(&f.expressions, Clone::clone),
        named_expressions: f.named_expressions.clone(),
        body: f.body.clone(),
    }
}

pub fn serde_range<T>(range: &std::ops::Range<u32>) -> naga::Range<T> {
    serde_json::from_str(serde_json::to_string(range).unwrap().as_str()).unwrap()
}

pub fn clone_arena<T, F: Fn(&T) -> T>(arena: &Arena<T>, f: F) -> Arena<T> {
    let mut into = Arena::new();
    for (_, t) in arena.iter() {
        into.append(f(t), Span::UNDEFINED);
    }

    into
}

pub fn clone_module(module: &Module) -> Module {
    let mut types = UniqueArena::new();
    for (_, ty) in module.types.iter() {
        types.insert(copy_type(ty), Span::UNDEFINED);
    }

    let mut entry_points = Vec::new();
    for ep in &module.entry_points {
        entry_points.push(EntryPoint {
            name: ep.name.clone(),
            stage: ep.stage,
            early_depth_test: ep.early_depth_test,
            workgroup_size: ep.workgroup_size,
            function: copy_func(&ep.function),
        })
    }

    Module {
        types,
        constants: clone_arena(&module.constants, clone_const),
        global_variables: clone_arena(&module.global_variables, Clone::clone),
        functions: clone_arena(&module.functions, copy_func),
        entry_points,
    }
}
