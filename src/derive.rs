use crate::util::copy_type;
use naga::{
    Arena, ArraySize, Block, Constant, ConstantInner, EntryPoint, Expression, Function,
    FunctionArgument, FunctionResult, GlobalVariable, Handle, ImageQuery, LocalVariable, Module,
    Span, Statement, StructMember, SwitchCase, Type, TypeInner, UniqueArena,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct DerivedModule<'a> {
    shader: Option<&'a Module>,
    span_offset: usize,

    type_map: HashMap<Handle<Type>, Handle<Type>>,
    const_map: HashMap<Handle<Constant>, Handle<Constant>>,
    global_map: HashMap<Handle<GlobalVariable>, Handle<GlobalVariable>>,
    function_map: HashMap<String, Handle<Function>>,

    types: UniqueArena<Type>,
    constants: Arena<Constant>,
    globals: Arena<GlobalVariable>,
    functions: Arena<Function>,
}

impl<'a> DerivedModule<'a> {
    // set source context for import operations
    pub fn set_shader_source(&mut self, shader: &'a Module, span_offset: usize) {
        self.clear_shader_source();
        self.shader = Some(shader);
        self.span_offset = span_offset;
    }

    // detach source context
    pub fn clear_shader_source(&mut self) {
        self.shader = None;
        self.type_map.clear();
        self.const_map.clear();
        self.global_map.clear();
    }

    pub fn map_span(&self, span: Span) -> Span {
        let span = span.to_range();
        match span {
            Some(rng) => Span::new(
                (rng.start + self.span_offset) as u32,
                (rng.end + self.span_offset) as u32,
            ),
            None => Span::UNDEFINED,
        }
    }

    // remap a type from source context into our derived context
    pub fn import_type(&mut self, h_type: &Handle<Type>) -> Handle<Type> {
        self.rename_type(h_type, None)
    }

    // remap a type from source context into our derived context, and rename it
    pub fn rename_type(&mut self, h_type: &Handle<Type>, name: Option<String>) -> Handle<Type> {
        self.type_map.get(h_type).copied().unwrap_or_else(|| {
            let ty = self
                .shader
                .as_ref()
                .unwrap()
                .types
                .get_handle(*h_type)
                .unwrap();

            let name = match name {
                Some(name) => Some(name),
                None => ty.name.clone(),
            };

            let new_type = Type {
                name,
                inner: match &ty.inner {
                    TypeInner::Scalar { .. }
                    | TypeInner::Vector { .. }
                    | TypeInner::Matrix { .. }
                    | TypeInner::ValuePointer { .. }
                    | TypeInner::Image { .. }
                    | TypeInner::Sampler { .. }
                    | TypeInner::Atomic { .. } => copy_type(ty).inner,

                    TypeInner::Pointer { base, space } => TypeInner::Pointer {
                        base: self.import_type(base),
                        space: *space,
                    },
                    TypeInner::Struct { members, span } => {
                        let members = members
                            .iter()
                            .map(|m| StructMember {
                                name: m.name.clone(),
                                ty: self.import_type(&m.ty),
                                binding: m.binding.clone(),
                                offset: m.offset,
                            })
                            .collect();
                        TypeInner::Struct {
                            members,
                            span: *span,
                        }
                    }
                    TypeInner::Array { base, size, stride } => {
                        let size = match size {
                            ArraySize::Constant(c) => ArraySize::Constant(self.import_const(c)),
                            ArraySize::Dynamic => ArraySize::Dynamic,
                        };
                        TypeInner::Array {
                            base: self.import_type(base),
                            size,
                            stride: *stride,
                        }
                    }
                    TypeInner::BindingArray { base, size } => {
                        let size = match size {
                            ArraySize::Constant(c) => ArraySize::Constant(self.import_const(c)),
                            ArraySize::Dynamic => ArraySize::Dynamic,
                        };
                        TypeInner::BindingArray {
                            base: self.import_type(base),
                            size,
                        }
                    }
                },
            };
            let span = self.shader.as_ref().unwrap().types.get_span(*h_type);
            let new_h = self.types.insert(new_type, self.map_span(span));
            self.type_map.insert(*h_type, new_h);
            new_h
        })
    }

    // remap a const from source context into our derived context
    pub fn import_const(&mut self, h_const: &Handle<Constant>) -> Handle<Constant> {
        self.const_map.get(h_const).copied().unwrap_or_else(|| {
            let c = self
                .shader
                .as_ref()
                .unwrap()
                .constants
                .try_get(*h_const)
                .unwrap();

            let new_const = Constant {
                name: c.name.clone(),
                specialization: c.specialization,
                inner: match &c.inner {
                    ConstantInner::Scalar { .. } => c.inner.clone(),
                    ConstantInner::Composite { ty, components } => {
                        let components = components.iter().map(|c| self.import_const(c)).collect();
                        ConstantInner::Composite {
                            ty: self.import_type(ty),
                            components,
                        }
                    }
                },
            };

            let span = self.shader.as_ref().unwrap().constants.get_span(*h_const);
            let new_h = self
                .constants
                .fetch_or_append(new_const, self.map_span(span));
            self.const_map.insert(*h_const, new_h);
            new_h
        })
    }

    // remap a global from source context into our derived context
    pub fn import_global(&mut self, h_global: &Handle<GlobalVariable>) -> Handle<GlobalVariable> {
        self.global_map.get(h_global).copied().unwrap_or_else(|| {
            let gv = self
                .shader
                .as_ref()
                .unwrap()
                .global_variables
                .try_get(*h_global)
                .unwrap();

            let new_global = GlobalVariable {
                name: gv.name.clone(),
                space: gv.space,
                binding: gv.binding.clone(),
                ty: self.import_type(&gv.ty),
                init: gv.init.map(|c| self.import_const(&c)),
            };

            let span = self
                .shader
                .as_ref()
                .unwrap()
                .global_variables
                .get_span(*h_global);
            let new_h = self
                .globals
                .fetch_or_append(new_global, self.map_span(span));
            self.global_map.insert(*h_global, new_h);
            new_h
        })
    }

    // remap a block
    fn import_block(
        &mut self,
        block: &Block,
        old_expressions: &Arena<Expression>,
        already_imported: &mut HashMap<Handle<Expression>, Handle<Expression>>,
        new_expressions: &mut Arena<Expression>,
    ) -> Block {
        let statements = block
            .iter()
            .map(|stmt| {
                match stmt {
                    // remap function calls
                    Statement::Call {
                        function,
                        arguments,
                        result,
                    } => Statement::Call {
                        function: self.map_function_handle(function),
                        arguments: arguments
                            .iter()
                            .map(|expr| {
                                self.import_expression(
                                    *expr,
                                    old_expressions,
                                    already_imported,
                                    new_expressions,
                                    false,
                                )
                            })
                            .collect(),
                        result: result.as_ref().map(|result| {
                            self.import_expression(
                                *result,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                false,
                            )
                        }),
                    },

                    // recursively
                    Statement::Block(b) => Statement::Block(self.import_block(
                        b,
                        old_expressions,
                        already_imported,
                        new_expressions,
                    )),
                    Statement::If {
                        condition,
                        accept,
                        reject,
                    } => Statement::If {
                        condition: self.import_expression(
                            *condition,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        accept: self.import_block(
                            accept,
                            old_expressions,
                            already_imported,
                            new_expressions,
                        ),
                        reject: self.import_block(
                            reject,
                            old_expressions,
                            already_imported,
                            new_expressions,
                        ),
                    },
                    Statement::Switch { selector, cases } => Statement::Switch {
                        selector: self.import_expression(
                            *selector,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        cases: cases
                            .iter()
                            .map(|case| SwitchCase {
                                value: case.value.clone(),
                                body: self.import_block(
                                    &case.body,
                                    old_expressions,
                                    already_imported,
                                    new_expressions,
                                ),
                                fall_through: case.fall_through,
                            })
                            .collect(),
                    },
                    Statement::Loop {
                        body,
                        continuing,
                        break_if,
                    } => Statement::Loop {
                        body: self.import_block(
                            body,
                            old_expressions,
                            already_imported,
                            new_expressions,
                        ),
                        continuing: self.import_block(
                            continuing,
                            old_expressions,
                            already_imported,
                            new_expressions,
                        ),
                        break_if: break_if.as_ref().map(|break_if| {
                            self.import_expression(
                                *break_if,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                false,
                            )
                        }),
                    },

                    // map expressions
                    Statement::Emit(exprs) => {
                        // iterate once to add expressions that should NOT be part of the emit statement
                        for expr in exprs.clone() {
                            self.import_expression(
                                expr,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                true,
                            );
                        }
                        let old_length = new_expressions.len();
                        // iterate again to add expressions that should be part of the emit statement
                        for expr in exprs.clone() {
                            self.import_expression(
                                expr,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                false,
                            );
                        }

                        Statement::Emit(new_expressions.range_from(old_length))
                    }
                    Statement::Store { pointer, value } => Statement::Store {
                        pointer: self.import_expression(
                            *pointer,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        value: self.import_expression(
                            *value,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                    },
                    Statement::ImageStore {
                        image,
                        coordinate,
                        array_index,
                        value,
                    } => Statement::ImageStore {
                        image: self.import_expression(
                            *image,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        coordinate: self.import_expression(
                            *coordinate,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        array_index: array_index.as_ref().map(|array_index| {
                            self.import_expression(
                                *array_index,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                false,
                            )
                        }),
                        value: self.import_expression(
                            *value,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                    },
                    Statement::Atomic {
                        pointer,
                        fun,
                        value,
                        result,
                    } => Statement::Atomic {
                        pointer: self.import_expression(
                            *pointer,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        fun: *fun,
                        value: self.import_expression(
                            *value,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                        result: self.import_expression(
                            *result,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            false,
                        ),
                    },
                    Statement::Return { value } => Statement::Return {
                        value: value.as_ref().map(|value| {
                            self.import_expression(
                                *value,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                false,
                            )
                        }),
                    },

                    // else just copy
                    Statement::Break
                    | Statement::Continue
                    | Statement::Kill
                    | Statement::Barrier(_) => stmt.clone(),
                }
            })
            .collect();

        let mut new_block = Block::from_vec(statements);

        for ((_, new_span), (_, old_span)) in new_block.span_iter_mut().zip(block.span_iter()) {
            *new_span.unwrap() = self.map_span(*old_span);
        }

        new_block
    }

    fn import_expression(
        &mut self,
        h_expr: Handle<Expression>,
        old_expressions: &Arena<Expression>,
        already_imported: &mut HashMap<Handle<Expression>, Handle<Expression>>,
        new_expressions: &mut Arena<Expression>,
        non_emitting_only: bool, // only brings items that should NOT be emitted into scope
    ) -> Handle<Expression> {
        if let Some(h_new) = already_imported.get(&h_expr) {
            return *h_new;
        }

        let mut is_external = false;
        let expr = old_expressions.try_get(h_expr).unwrap();
        let expr = match expr {
            Expression::CallResult(f) => Expression::CallResult(self.map_function_handle(f)),
            Expression::Constant(c) => {
                is_external = true;
                Expression::Constant(self.import_const(c))
            }
            Expression::Compose { ty, components } => Expression::Compose {
                ty: self.import_type(ty),
                components: components
                    .iter()
                    .map(|expr| {
                        self.import_expression(
                            *expr,
                            old_expressions,
                            already_imported,
                            new_expressions,
                            non_emitting_only,
                        )
                    })
                    .collect(),
            },
            Expression::GlobalVariable(gv) => {
                is_external = true;
                Expression::GlobalVariable(self.import_global(gv))
            }
            Expression::ImageSample {
                image,
                sampler,
                gather,
                coordinate,
                array_index,
                offset,
                level,
                depth_ref,
            } => Expression::ImageSample {
                image: self.import_expression(
                    *image,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                sampler: self.import_expression(
                    *sampler,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                gather: *gather,
                coordinate: self.import_expression(
                    *coordinate,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                array_index: array_index.as_ref().map(|array_index| {
                    self.import_expression(
                        *array_index,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
                offset: offset.map(|c| self.import_const(&c)),
                level: *level,
                depth_ref: depth_ref.as_ref().map(|depth_ref| {
                    self.import_expression(
                        *depth_ref,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
            },
            Expression::Access { base, index } => Expression::Access {
                base: self.import_expression(
                    *base,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                index: self.import_expression(
                    *index,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::AccessIndex { base, index } => Expression::AccessIndex {
                base: self.import_expression(
                    *base,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                index: *index,
            },
            Expression::Splat { size, value } => Expression::Splat {
                size: *size,
                value: self.import_expression(
                    *value,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Swizzle {
                size,
                vector,
                pattern,
            } => Expression::Swizzle {
                size: *size,
                vector: self.import_expression(
                    *vector,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                pattern: *pattern,
            },
            Expression::Load { pointer } => Expression::Load {
                pointer: self.import_expression(
                    *pointer,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => Expression::ImageLoad {
                image: self.import_expression(
                    *image,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                coordinate: self.import_expression(
                    *coordinate,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                array_index: array_index.as_ref().map(|array_index| {
                    self.import_expression(
                        *array_index,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
                sample: sample.as_ref().map(|sample| {
                    self.import_expression(
                        *sample,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
                level: level.as_ref().map(|level| {
                    self.import_expression(
                        *level,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
            },
            Expression::ImageQuery { image, query } => Expression::ImageQuery {
                image: self.import_expression(
                    *image,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                query: match query {
                    ImageQuery::Size { level } => ImageQuery::Size {
                        level: level.as_ref().map(|level| {
                            self.import_expression(
                                *level,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                non_emitting_only,
                            )
                        }),
                    },
                    _ => *query,
                },
            },
            Expression::Unary { op, expr } => Expression::Unary {
                op: *op,
                expr: self.import_expression(
                    *expr,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: self.import_expression(
                    *left,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                right: self.import_expression(
                    *right,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Select {
                condition,
                accept,
                reject,
            } => Expression::Select {
                condition: self.import_expression(
                    *condition,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                accept: self.import_expression(
                    *accept,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                reject: self.import_expression(
                    *reject,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Derivative { axis, expr } => Expression::Derivative {
                axis: *axis,
                expr: self.import_expression(
                    *expr,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Relational { fun, argument } => Expression::Relational {
                fun: *fun,
                argument: self.import_expression(
                    *argument,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
            },
            Expression::Math {
                fun,
                arg,
                arg1,
                arg2,
                arg3,
            } => Expression::Math {
                fun: *fun,
                arg: self.import_expression(
                    *arg,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                arg1: arg1.as_ref().map(|arg| {
                    self.import_expression(
                        *arg,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
                arg2: arg2.as_ref().map(|arg| {
                    self.import_expression(
                        *arg,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
                arg3: arg3.as_ref().map(|arg| {
                    self.import_expression(
                        *arg,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                }),
            },
            Expression::As {
                expr,
                kind,
                convert,
            } => Expression::As {
                expr: self.import_expression(
                    *expr,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                ),
                kind: *kind,
                convert: *convert,
            },
            Expression::ArrayLength(expr) => Expression::ArrayLength(self.import_expression(
                *expr,
                old_expressions,
                already_imported,
                new_expressions,
                non_emitting_only,
            )),

            Expression::LocalVariable(_) | Expression::FunctionArgument(_) => {
                is_external = true;
                expr.clone()
            }

            Expression::AtomicResult { .. } => expr.clone(),
        };

        if !non_emitting_only || is_external {
            let span = old_expressions.get_span(h_expr);
            let h_new = new_expressions.append(expr, self.map_span(span));

            already_imported.insert(h_expr, h_new);
            h_new
        } else {
            h_expr
        }
    }

    // remap function global references (global vars, consts, types) into our derived context
    pub fn localize_function(&mut self, func: &Function) -> Function {
        let arguments = func
            .arguments
            .iter()
            .map(|arg| FunctionArgument {
                name: arg.name.clone(),
                ty: self.import_type(&arg.ty),
                binding: arg.binding.clone(),
            })
            .collect();

        let result = func.result.as_ref().map(|r| FunctionResult {
            ty: self.import_type(&r.ty),
            binding: r.binding.clone(),
        });

        let mut local_variables = Arena::new();
        for (h_l, l) in func.local_variables.iter() {
            let new_local = LocalVariable {
                name: l.name.clone(),
                ty: self.import_type(&l.ty),
                init: l.init.map(|c| self.import_const(&c)),
            };
            let span = func.local_variables.get_span(h_l);
            let new_h = local_variables.append(new_local, self.map_span(span));
            assert_eq!(h_l, new_h);
        }

        let mut expressions = Arena::new();
        let mut expr_map = HashMap::new();

        let body = self.import_block(
            &func.body,
            &func.expressions,
            &mut expr_map,
            &mut expressions,
        );

        Function {
            name: func.name.clone(),
            arguments,
            result,
            local_variables,
            expressions,
            named_expressions: func.named_expressions.clone(),
            body,
        }
    }

    // import a function defined in the source shader context.
    // func name may be already defined, the returned handle will refer to the new function.
    // the previously defined function will still be valid.
    pub fn import_function(&mut self, func: &Function, span: Span) -> Handle<Function> {
        let name = func.name.as_ref().unwrap().clone();
        let mapped_func = self.localize_function(func);
        let new_span = self.map_span(span);
        let new_h = self.functions.append(mapped_func, new_span);
        self.function_map.insert(name, new_h);
        new_h
    }

    // get the derived handle corresponding to the given source function handle
    // requires func to be named
    pub fn map_function_handle(&self, h_func: &Handle<Function>) -> Handle<Function> {
        let name = self
            .shader
            .as_ref()
            .unwrap()
            .functions
            .try_get(*h_func)
            .unwrap()
            .name
            .as_ref()
            .unwrap();
        *self.function_map.get(name).unwrap()
    }

    /// swap an already imported function for a new one.
    /// note span cannot be updated
    pub fn import_function_if_new(&mut self, func: &Function, span: Span) -> Handle<Function> {
        let name = func.name.as_ref().unwrap().clone();
        if let Some(h) = self.function_map.get(&name) {
            return *h;
        }

        self.import_function(func, span)
    }

    pub fn into_module_with_entrypoints(mut self) -> naga::Module {
        let entry_points = self
            .shader
            .unwrap()
            .entry_points
            .iter()
            .map(|ep| EntryPoint {
                name: ep.name.clone(),
                stage: ep.stage,
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
                function: self.localize_function(&ep.function),
            })
            .collect();

        naga::Module {
            entry_points,
            ..self.into()
        }
    }
}

impl<'a> From<DerivedModule<'a>> for naga::Module {
    fn from(derived: DerivedModule) -> Self {
        naga::Module {
            types: derived.types,
            constants: derived.constants,
            global_variables: derived.globals,
            functions: derived.functions,
            entry_points: Default::default(),
        }
    }
}
