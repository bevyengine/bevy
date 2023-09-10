use naga::Expression;

// Expression does not implement PartialEq except for internal testing (cfg_attr(test)), so we must use our own version.
// This implementation is tweaked from the output of `cargo expand`
#[inline]
pub fn expression_eq(lhs: &Expression, rhs: &Expression) -> bool {
    let __lhs_tag = std::mem::discriminant(lhs);
    let __arg1_tag = std::mem::discriminant(rhs);
    __lhs_tag == __arg1_tag
        && match (lhs, rhs) {
            (Expression::Literal(__lhs_0), Expression::Literal(__arg1_0)) => *__lhs_0 == *__arg1_0,
            (Expression::Constant(__lhs_0), Expression::Constant(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (Expression::ZeroValue(__lhs_0), Expression::ZeroValue(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (
                Expression::Compose {
                    ty: __lhs_0,
                    components: __lhs_1,
                },
                Expression::Compose {
                    ty: __arg1_0,
                    components: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Access {
                    base: __lhs_0,
                    index: __lhs_1,
                },
                Expression::Access {
                    base: __arg1_0,
                    index: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::AccessIndex {
                    base: __lhs_0,
                    index: __lhs_1,
                },
                Expression::AccessIndex {
                    base: __arg1_0,
                    index: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Splat {
                    size: __lhs_0,
                    value: __lhs_1,
                },
                Expression::Splat {
                    size: __arg1_0,
                    value: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Swizzle {
                    size: __lhs_0,
                    vector: __lhs_1,
                    pattern: __lhs_2,
                },
                Expression::Swizzle {
                    size: __arg1_0,
                    vector: __arg1_1,
                    pattern: __arg1_2,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1 && *__lhs_2 == *__arg1_2,
            (Expression::FunctionArgument(__lhs_0), Expression::FunctionArgument(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (Expression::GlobalVariable(__lhs_0), Expression::GlobalVariable(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (Expression::LocalVariable(__lhs_0), Expression::LocalVariable(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (Expression::Load { pointer: __lhs_0 }, Expression::Load { pointer: __arg1_0 }) => {
                *__lhs_0 == *__arg1_0
            }
            (
                Expression::ImageSample {
                    image: __lhs_0,
                    sampler: __lhs_1,
                    gather: __lhs_2,
                    coordinate: __lhs_3,
                    array_index: __lhs_4,
                    offset: __lhs_5,
                    level: __lhs_6,
                    depth_ref: __lhs_7,
                },
                Expression::ImageSample {
                    image: __arg1_0,
                    sampler: __arg1_1,
                    gather: __arg1_2,
                    coordinate: __arg1_3,
                    array_index: __arg1_4,
                    offset: __arg1_5,
                    level: __arg1_6,
                    depth_ref: __arg1_7,
                },
            ) => {
                *__lhs_0 == *__arg1_0
                    && *__lhs_1 == *__arg1_1
                    && *__lhs_2 == *__arg1_2
                    && *__lhs_3 == *__arg1_3
                    && *__lhs_4 == *__arg1_4
                    && *__lhs_5 == *__arg1_5
                    && *__lhs_6 == *__arg1_6
                    && *__lhs_7 == *__arg1_7
            }
            (
                Expression::ImageLoad {
                    image: __lhs_0,
                    coordinate: __lhs_1,
                    array_index: __lhs_2,
                    sample: __lhs_3,
                    level: __lhs_4,
                },
                Expression::ImageLoad {
                    image: __arg1_0,
                    coordinate: __arg1_1,
                    array_index: __arg1_2,
                    sample: __arg1_3,
                    level: __arg1_4,
                },
            ) => {
                *__lhs_0 == *__arg1_0
                    && *__lhs_1 == *__arg1_1
                    && *__lhs_2 == *__arg1_2
                    && *__lhs_3 == *__arg1_3
                    && *__lhs_4 == *__arg1_4
            }
            (
                Expression::ImageQuery {
                    image: __lhs_0,
                    query: __lhs_1,
                },
                Expression::ImageQuery {
                    image: __arg1_0,
                    query: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Unary {
                    op: __lhs_0,
                    expr: __lhs_1,
                },
                Expression::Unary {
                    op: __arg1_0,
                    expr: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Binary {
                    op: __lhs_0,
                    left: __lhs_1,
                    right: __lhs_2,
                },
                Expression::Binary {
                    op: __arg1_0,
                    left: __arg1_1,
                    right: __arg1_2,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1 && *__lhs_2 == *__arg1_2,
            (
                Expression::Select {
                    condition: __lhs_0,
                    accept: __lhs_1,
                    reject: __lhs_2,
                },
                Expression::Select {
                    condition: __arg1_0,
                    accept: __arg1_1,
                    reject: __arg1_2,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1 && *__lhs_2 == *__arg1_2,
            (
                Expression::Derivative {
                    axis: __lhs_0,
                    ctrl: __lhs_1,
                    expr: __lhs_2,
                },
                Expression::Derivative {
                    axis: __arg1_0,
                    ctrl: __arg1_1,
                    expr: __arg1_2,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1 && *__lhs_2 == *__arg1_2,
            (
                Expression::Relational {
                    fun: __lhs_0,
                    argument: __lhs_1,
                },
                Expression::Relational {
                    fun: __arg1_0,
                    argument: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::Math {
                    fun: __lhs_0,
                    arg: __lhs_1,
                    arg1: __lhs_2,
                    arg2: __lhs_3,
                    arg3: __lhs_4,
                },
                Expression::Math {
                    fun: __arg1_0,
                    arg: __arg1_1,
                    arg1: __arg1_2,
                    arg2: __arg1_3,
                    arg3: __arg1_4,
                },
            ) => {
                *__lhs_0 == *__arg1_0
                    && *__lhs_1 == *__arg1_1
                    && *__lhs_2 == *__arg1_2
                    && *__lhs_3 == *__arg1_3
                    && *__lhs_4 == *__arg1_4
            }
            (
                Expression::As {
                    expr: __lhs_0,
                    kind: __lhs_1,
                    convert: __lhs_2,
                },
                Expression::As {
                    expr: __arg1_0,
                    kind: __arg1_1,
                    convert: __arg1_2,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1 && *__lhs_2 == *__arg1_2,
            (Expression::CallResult(__lhs_0), Expression::CallResult(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (
                Expression::AtomicResult {
                    ty: __lhs_0,
                    comparison: __lhs_1,
                },
                Expression::AtomicResult {
                    ty: __arg1_0,
                    comparison: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            (
                Expression::WorkGroupUniformLoadResult { ty: __lhs_0 },
                Expression::WorkGroupUniformLoadResult { ty: __arg1_0 },
            ) => *__lhs_0 == *__arg1_0,
            (Expression::ArrayLength(__lhs_0), Expression::ArrayLength(__arg1_0)) => {
                *__lhs_0 == *__arg1_0
            }
            (
                Expression::RayQueryGetIntersection {
                    query: __lhs_0,
                    committed: __lhs_1,
                },
                Expression::RayQueryGetIntersection {
                    query: __arg1_0,
                    committed: __arg1_1,
                },
            ) => *__lhs_0 == *__arg1_0 && *__lhs_1 == *__arg1_1,
            _ => unreachable!(),
        }
}
