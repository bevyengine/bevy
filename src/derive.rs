use indexmap::IndexMap;
use naga::{
    Arena, AtomicFunction, Block, Constant, EntryPoint, Expression, Function, FunctionArgument,
    FunctionResult, GlobalVariable, Handle, ImageQuery, LocalVariable, Module, SampleLevel, Span,
    Statement, StructMember, SwitchCase, Type, TypeInner, UniqueArena,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Default)]
pub struct DerivedModule<'a> {
    shader: Option<&'a Module>,
    span_offset: usize,

    type_map: IndexMap<Handle<Type>, Handle<Type>>,
    const_map: IndexMap<Handle<Constant>, Handle<Constant>>,
    const_expression_map: Rc<RefCell<IndexMap<Handle<Expression>, Handle<Expression>>>>,
    global_map: IndexMap<Handle<GlobalVariable>, Handle<GlobalVariable>>,
    function_map: IndexMap<String, Handle<Function>>,

    types: UniqueArena<Type>,
    constants: Arena<Constant>,
    const_expressions: Rc<RefCell<Arena<Expression>>>,
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
        self.const_expression_map.borrow_mut().clear();
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
                    | TypeInner::Atomic { .. }
                    | TypeInner::AccelerationStructure
                    | TypeInner::RayQuery => ty.inner.clone(),

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
                    TypeInner::Array { base, size, stride } => TypeInner::Array {
                        base: self.import_type(base),
                        size: *size,
                        stride: *stride,
                    },
                    TypeInner::BindingArray { base, size } => TypeInner::BindingArray {
                        base: self.import_type(base),
                        size: *size,
                    },
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
                r#override: c.r#override.clone(),
                ty: self.import_type(&c.ty),
                init: self.import_const_expression(c.init),
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
                init: gv.init.map(|c| self.import_const_expression(c)),
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
    // remap a const expression from source context into our derived context
    pub fn import_const_expression(&mut self, h_cexpr: Handle<Expression>) -> Handle<Expression> {
        self.import_expression(
            h_cexpr,
            &self.shader.as_ref().unwrap().const_expressions,
            self.const_expression_map.clone(),
            self.const_expressions.clone(),
            false,
            true,
        )
    }

    // remap a block
    fn import_block(
        &mut self,
        block: &Block,
        old_expressions: &Arena<Expression>,
        already_imported: Rc<RefCell<IndexMap<Handle<Expression>, Handle<Expression>>>>,
        new_expressions: Rc<RefCell<Arena<Expression>>>,
    ) -> Block {
        macro_rules! map_expr {
            ($e:expr) => {
                self.import_expression(
                    *$e,
                    old_expressions,
                    already_imported.clone(),
                    new_expressions.clone(),
                    false,
                    false,
                )
            };
        }

        macro_rules! map_expr_opt {
            ($e:expr) => {
                $e.as_ref().map(|expr| map_expr!(expr))
            };
        }

        macro_rules! map_block {
            ($b:expr) => {
                self.import_block(
                    $b,
                    old_expressions,
                    already_imported.clone(),
                    new_expressions.clone(),
                )
            };
        }

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
                        arguments: arguments.iter().map(|expr| map_expr!(expr)).collect(),
                        result: result.as_ref().map(|result| map_expr!(result)),
                    },

                    // recursively
                    Statement::Block(b) => Statement::Block(map_block!(b)),
                    Statement::If {
                        condition,
                        accept,
                        reject,
                    } => Statement::If {
                        condition: map_expr!(condition),
                        accept: map_block!(accept),
                        reject: map_block!(reject),
                    },
                    Statement::Switch { selector, cases } => Statement::Switch {
                        selector: map_expr!(selector),
                        cases: cases
                            .iter()
                            .map(|case| SwitchCase {
                                value: case.value,
                                body: map_block!(&case.body),
                                fall_through: case.fall_through,
                            })
                            .collect(),
                    },
                    Statement::Loop {
                        body,
                        continuing,
                        break_if,
                    } => Statement::Loop {
                        body: map_block!(body),
                        continuing: map_block!(continuing),
                        break_if: map_expr_opt!(break_if),
                    },

                    // map expressions
                    Statement::Emit(exprs) => {
                        // iterate once to add expressions that should NOT be part of the emit statement
                        for expr in exprs.clone() {
                            self.import_expression(
                                expr,
                                old_expressions,
                                already_imported.clone(),
                                new_expressions.clone(),
                                true,
                                false,
                            );
                        }
                        let old_length = new_expressions.borrow().len();
                        // iterate again to add expressions that should be part of the emit statement
                        for expr in exprs.clone() {
                            map_expr!(&expr);
                        }

                        Statement::Emit(new_expressions.borrow().range_from(old_length))
                    }
                    Statement::Store { pointer, value } => Statement::Store {
                        pointer: map_expr!(pointer),
                        value: map_expr!(value),
                    },
                    Statement::ImageStore {
                        image,
                        coordinate,
                        array_index,
                        value,
                    } => Statement::ImageStore {
                        image: map_expr!(image),
                        coordinate: map_expr!(coordinate),
                        array_index: map_expr_opt!(array_index),
                        value: map_expr!(value),
                    },
                    Statement::Atomic {
                        pointer,
                        fun,
                        value,
                        result,
                    } => {
                        let fun = match fun {
                            AtomicFunction::Exchange {
                                compare: Some(compare_expr),
                            } => AtomicFunction::Exchange {
                                compare: Some(map_expr!(compare_expr)),
                            },
                            fun => *fun,
                        };
                        Statement::Atomic {
                            pointer: map_expr!(pointer),
                            fun,
                            value: map_expr!(value),
                            result: map_expr!(result),
                        }
                    }
                    Statement::WorkGroupUniformLoad { pointer, result } => {
                        Statement::WorkGroupUniformLoad {
                            pointer: map_expr!(pointer),
                            result: map_expr!(result),
                        }
                    }
                    Statement::Return { value } => Statement::Return {
                        value: map_expr_opt!(value),
                    },
                    Statement::RayQuery { query, fun } => Statement::RayQuery {
                        query: map_expr!(query),
                        fun: match fun {
                            naga::RayQueryFunction::Initialize {
                                acceleration_structure,
                                descriptor,
                            } => naga::RayQueryFunction::Initialize {
                                acceleration_structure: map_expr!(acceleration_structure),
                                descriptor: map_expr!(descriptor),
                            },
                            naga::RayQueryFunction::Proceed { result } => {
                                naga::RayQueryFunction::Proceed {
                                    result: map_expr!(result),
                                }
                            }
                            naga::RayQueryFunction::Terminate => naga::RayQueryFunction::Terminate,
                        },
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
        already_imported: Rc<RefCell<IndexMap<Handle<Expression>, Handle<Expression>>>>,
        new_expressions: Rc<RefCell<Arena<Expression>>>,
        non_emitting_only: bool, // only brings items that should NOT be emitted into scope
        unique: bool,            // ensure expressions are unique with custom comparison
    ) -> Handle<Expression> {
        if let Some(h_new) = already_imported.borrow().get(&h_expr) {
            return *h_new;
        }

        macro_rules! map_expr {
            ($e:expr) => {
                self.import_expression(
                    *$e,
                    old_expressions,
                    already_imported.clone(),
                    new_expressions.clone(),
                    non_emitting_only,
                    unique,
                )
            };
        }

        macro_rules! map_expr_opt {
            ($e:expr) => {
                $e.as_ref().map(|expr| {
                    self.import_expression(
                        *expr,
                        old_expressions,
                        already_imported.clone(),
                        new_expressions.clone(),
                        non_emitting_only,
                        unique,
                    )
                })
            };
        }

        let mut is_external = false;
        let expr = old_expressions.try_get(h_expr).unwrap();
        let expr = match expr {
            Expression::Literal(_) => {
                is_external = true;
                expr.clone()
            }
            Expression::ZeroValue(zv) => {
                is_external = true;
                Expression::ZeroValue(self.import_type(zv))
            }
            Expression::CallResult(f) => Expression::CallResult(self.map_function_handle(f)),
            Expression::Constant(c) => {
                is_external = true;
                Expression::Constant(self.import_const(c))
            }
            Expression::Compose { ty, components } => Expression::Compose {
                ty: self.import_type(ty),
                components: components.iter().map(|expr| map_expr!(expr)).collect(),
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
                image: map_expr!(image),
                sampler: map_expr!(sampler),
                gather: *gather,
                coordinate: map_expr!(coordinate),
                array_index: map_expr_opt!(array_index),
                offset: offset.map(|c| self.import_const_expression(c)),
                level: match level {
                    SampleLevel::Auto | SampleLevel::Zero => *level,
                    SampleLevel::Exact(expr) => SampleLevel::Exact(map_expr!(expr)),
                    SampleLevel::Bias(expr) => SampleLevel::Bias(map_expr!(expr)),
                    SampleLevel::Gradient { x, y } => SampleLevel::Gradient {
                        x: map_expr!(x),
                        y: map_expr!(y),
                    },
                },
                depth_ref: map_expr_opt!(depth_ref),
            },
            Expression::Access { base, index } => Expression::Access {
                base: map_expr!(base),
                index: map_expr!(index),
            },
            Expression::AccessIndex { base, index } => Expression::AccessIndex {
                base: map_expr!(base),
                index: *index,
            },
            Expression::Splat { size, value } => Expression::Splat {
                size: *size,
                value: map_expr!(value),
            },
            Expression::Swizzle {
                size,
                vector,
                pattern,
            } => Expression::Swizzle {
                size: *size,
                vector: map_expr!(vector),
                pattern: *pattern,
            },
            Expression::Load { pointer } => Expression::Load {
                pointer: map_expr!(pointer),
            },
            Expression::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => Expression::ImageLoad {
                image: map_expr!(image),
                coordinate: map_expr!(coordinate),
                array_index: map_expr_opt!(array_index),
                sample: map_expr_opt!(sample),
                level: map_expr_opt!(level),
            },
            Expression::ImageQuery { image, query } => Expression::ImageQuery {
                image: map_expr!(image),
                query: match query {
                    ImageQuery::Size { level } => ImageQuery::Size {
                        level: map_expr_opt!(level),
                    },
                    _ => *query,
                },
            },
            Expression::Unary { op, expr } => Expression::Unary {
                op: *op,
                expr: map_expr!(expr),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: map_expr!(left),
                right: map_expr!(right),
            },
            Expression::Select {
                condition,
                accept,
                reject,
            } => Expression::Select {
                condition: map_expr!(condition),
                accept: map_expr!(accept),
                reject: map_expr!(reject),
            },
            Expression::Derivative { axis, expr, ctrl } => Expression::Derivative {
                axis: *axis,
                expr: map_expr!(expr),
                ctrl: *ctrl,
            },
            Expression::Relational { fun, argument } => Expression::Relational {
                fun: *fun,
                argument: map_expr!(argument),
            },
            Expression::Math {
                fun,
                arg,
                arg1,
                arg2,
                arg3,
            } => Expression::Math {
                fun: *fun,
                arg: map_expr!(arg),
                arg1: map_expr_opt!(arg1),
                arg2: map_expr_opt!(arg2),
                arg3: map_expr_opt!(arg3),
            },
            Expression::As {
                expr,
                kind,
                convert,
            } => Expression::As {
                expr: map_expr!(expr),
                kind: *kind,
                convert: *convert,
            },
            Expression::ArrayLength(expr) => Expression::ArrayLength(map_expr!(expr)),

            Expression::LocalVariable(_) | Expression::FunctionArgument(_) => {
                is_external = true;
                expr.clone()
            }

            Expression::AtomicResult { ty, comparison } => Expression::AtomicResult {
                ty: self.import_type(ty),
                comparison: *comparison,
            },
            Expression::WorkGroupUniformLoadResult { ty } => {
                Expression::WorkGroupUniformLoadResult {
                    ty: self.import_type(ty),
                }
            }
            Expression::RayQueryProceedResult => expr.clone(),
            Expression::RayQueryGetIntersection { query, committed } => {
                Expression::RayQueryGetIntersection {
                    query: map_expr!(query),
                    committed: *committed,
                }
            }
        };

        if !non_emitting_only || is_external {
            let span = old_expressions.get_span(h_expr);
            let h_new = if unique {
                new_expressions.borrow_mut().fetch_if_or_append(
                    expr,
                    self.map_span(span),
                    |lhs, rhs| lhs == rhs,
                )
            } else {
                new_expressions
                    .borrow_mut()
                    .append(expr, self.map_span(span))
            };

            already_imported.borrow_mut().insert(h_expr, h_new);
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

        let expressions = Rc::new(RefCell::new(Arena::new()));
        let expr_map = Rc::new(RefCell::new(IndexMap::new()));

        let mut local_variables = Arena::new();
        for (h_l, l) in func.local_variables.iter() {
            let new_local = LocalVariable {
                name: l.name.clone(),
                ty: self.import_type(&l.ty),
                init: l.init.map(|c| {
                    self.import_expression(
                        c,
                        &func.expressions,
                        expr_map.clone(),
                        expressions.clone(),
                        false,
                        true,
                    )
                }),
            };
            let span = func.local_variables.get_span(h_l);
            let new_h = local_variables.append(new_local, self.map_span(span));
            assert_eq!(h_l, new_h);
        }

        let body = self.import_block(
            &func.body,
            &func.expressions,
            expr_map.clone(),
            expressions.clone(),
        );

        let named_expressions = func
            .named_expressions
            .iter()
            .flat_map(|(h_expr, name)| {
                expr_map
                    .borrow()
                    .get(h_expr)
                    .map(|new_h| (*new_h, name.clone()))
            })
            .collect::<IndexMap<_, _, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>>();

        Function {
            name: func.name.clone(),
            arguments,
            result,
            local_variables,
            expressions: Rc::try_unwrap(expressions).unwrap().into_inner(),
            named_expressions,
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
    pub fn map_function_handle(&mut self, h_func: &Handle<Function>) -> Handle<Function> {
        let functions = &self.shader.as_ref().unwrap().functions;
        let func = functions.try_get(*h_func).unwrap();
        let name = func.name.as_ref().unwrap();
        self.function_map.get(name).copied().unwrap_or_else(|| {
            let span = functions.get_span(*h_func);
            self.import_function(func, span)
        })
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
            const_expressions: Rc::try_unwrap(derived.const_expressions)
                .unwrap()
                .into_inner(),
            functions: derived.functions,
            special_types: Default::default(),
            entry_points: Default::default(),
        }
    }
}
