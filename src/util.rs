// polyfill clones

// this does not remap type handles, only use if the types are not modified
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

pub fn serde_range<T>(range: &std::ops::Range<u32>) -> naga::Range<T> {
    serde_json::from_str(serde_json::to_string(range).unwrap().as_str()).unwrap()
}
