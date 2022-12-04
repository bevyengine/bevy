use crate::util::copy_type;
use naga::{
    Arena, ArraySize, Block, Constant, ConstantInner, EntryPoint, Expression, Function,
    FunctionArgument, FunctionResult, GlobalVariable, Handle, LocalVariable, Module, Span,
    Statement, StructMember, SwitchCase, Type, TypeInner, UniqueArena,
};
use std::collections::{HashMap, HashSet};

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
    fn import_block(&self, block: &Block) -> Block {
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
                        arguments: arguments.clone(),
                        result: *result,
                    },

                    // recursively
                    Statement::Block(b) => Statement::Block(self.import_block(b)),
                    Statement::If {
                        condition,
                        accept,
                        reject,
                    } => Statement::If {
                        condition: *condition,
                        accept: self.import_block(accept),
                        reject: self.import_block(reject),
                    },
                    Statement::Switch { selector, cases } => Statement::Switch {
                        selector: *selector,
                        cases: cases
                            .iter()
                            .map(|case| SwitchCase {
                                value: case.value.clone(),
                                body: self.import_block(&case.body),
                                fall_through: case.fall_through,
                            })
                            .collect(),
                    },
                    Statement::Loop {
                        body,
                        continuing,
                        break_if,
                    } => Statement::Loop {
                        body: self.import_block(body),
                        continuing: self.import_block(continuing),
                        break_if: *break_if,
                    },

                    // else copy
                    Statement::Emit(_)
                    | Statement::Break
                    | Statement::Continue
                    | Statement::Return { .. }
                    | Statement::Kill
                    | Statement::Barrier(_)
                    | Statement::Store { .. }
                    | Statement::ImageStore { .. }
                    | Statement::Atomic { .. } => stmt.clone(),
                }
            })
            .collect();

        let mut new_block = Block::from_vec(statements);

        for ((_, new_span), (_, old_span)) in new_block.span_iter_mut().zip(block.span_iter()) {
            *new_span.unwrap() = self.map_span(*old_span);
        }

        new_block
    }

    fn gather_expr_used_expressions(expr: Handle<Expression>, exprs: &Arena<Expression>, into: &mut HashSet<Handle<Expression>>) {
        into.insert(expr);
        let expr = exprs.try_get(expr).unwrap();
        match expr {
            Expression::Access { base, index } => {
                Self::gather_expr_used_expressions(*base, exprs, into);
                Self::gather_expr_used_expressions(*index, exprs, into);
            },
            Expression::AccessIndex { base, .. } => {
                Self::gather_expr_used_expressions(*base, exprs, into);
            }
            Expression::Splat { value, .. } => {
                Self::gather_expr_used_expressions(*value, exprs, into);
            },
            Expression::Swizzle { vector, .. } => {
                Self::gather_expr_used_expressions(*vector, exprs, into);
            },
            Expression::Compose { components, .. } => {
                for component in components {
                    Self::gather_expr_used_expressions(*component, exprs, into);
                }
            },
            Expression::Load { pointer } => {
                Self::gather_expr_used_expressions(*pointer, exprs, into);
            }
            Expression::ImageSample { image, sampler, coordinate, array_index, depth_ref, .. } => {
                Self::gather_expr_used_expressions(*image, exprs, into);
                Self::gather_expr_used_expressions(*sampler, exprs, into);
                Self::gather_expr_used_expressions(*coordinate, exprs, into);
                array_index.map(|array_index| Self::gather_expr_used_expressions(array_index, exprs, into));
                depth_ref.map(|depth_ref| Self::gather_expr_used_expressions(depth_ref, exprs, into)); 
            },
            Expression::ImageLoad { image, coordinate, array_index, sample, level } => {
                Self::gather_expr_used_expressions(*image, exprs, into);
                Self::gather_expr_used_expressions(*coordinate, exprs, into);
                array_index.map(|array_index| Self::gather_expr_used_expressions(array_index, exprs, into));
                sample.map(|sample| Self::gather_expr_used_expressions(sample, exprs, into));
                level.map(|level| Self::gather_expr_used_expressions(level, exprs, into));
            },
            Expression::ImageQuery { image, .. } => {
                Self::gather_expr_used_expressions(*image, exprs, into);
            }
            Expression::Binary { left, right, .. } => {
                Self::gather_expr_used_expressions(*left, exprs, into);
                Self::gather_expr_used_expressions(*right, exprs, into);
            }
            Expression::Select { condition, accept, reject } => {
                Self::gather_expr_used_expressions(*condition, exprs, into);
                Self::gather_expr_used_expressions(*accept, exprs, into);
                Self::gather_expr_used_expressions(*reject, exprs, into);                
            },
            Expression::Relational { argument, .. } => {
                Self::gather_expr_used_expressions(*argument, exprs, into);
            },
            Expression::Math { arg, arg1, arg2, arg3, .. } => {
                Self::gather_expr_used_expressions(*arg, exprs, into);
                arg1.map(|arg| Self::gather_expr_used_expressions(arg, exprs, into));
                arg2.map(|arg| Self::gather_expr_used_expressions(arg, exprs, into));
                arg3.map(|arg| Self::gather_expr_used_expressions(arg, exprs, into));
            },
            Expression::Unary { expr, .. } |
            Expression::Derivative { expr, .. } |
            Expression::As { expr, .. } |
            Expression::ArrayLength(expr) => {
                Self::gather_expr_used_expressions(*expr, exprs, into);
            },
            Expression::AtomicResult { .. } |
            Expression::CallResult(_) |
            Expression::Constant(_) |
            Expression::FunctionArgument(_) |
            Expression::GlobalVariable(_) |
            Expression::LocalVariable(_) => (),
        }
    }

    fn gather_block_used_expressions(block: &Block, exprs: &Arena<Expression>, into: &mut HashSet<Handle<Expression>>) {
        for stmt in block {
            match stmt {
                Statement::Emit(range) => {
                    for h in range.clone() {
                        Self::gather_expr_used_expressions(h, exprs, into);
                    }
                },
                Statement::Block(b) => Self::gather_block_used_expressions(b, exprs, into),
                Statement::If { condition, accept, reject } => {
                    Self::gather_expr_used_expressions(*condition, exprs, into);
                    Self::gather_block_used_expressions(accept, exprs, into);
                    Self::gather_block_used_expressions(reject, exprs, into);
                }
                Statement::Switch { selector, cases } => {
                    Self::gather_expr_used_expressions(*selector, exprs, into);
                    for case in cases {
                        Self::gather_block_used_expressions(&case.body, exprs, into);
                    }
                },
                Statement::Loop { body, continuing, break_if } => {
                    break_if.map(|break_if| into.insert(break_if));
                    Self::gather_block_used_expressions(body, exprs, into);
                    Self::gather_block_used_expressions(continuing, exprs, into);
                },
                Statement::Return { value } => {
                    value.map(|value| Self::gather_expr_used_expressions(value, exprs, into));
                },
                Statement::Store { pointer, value } => {
                    Self::gather_expr_used_expressions(*pointer, exprs, into);
                    Self::gather_expr_used_expressions(*value, exprs, into);
                },
                Statement::ImageStore { image, coordinate, array_index, value } => {
                    Self::gather_expr_used_expressions(*image, exprs, into);                    
                    Self::gather_expr_used_expressions(*coordinate, exprs, into);
                    array_index.map(|array_index| Self::gather_expr_used_expressions(array_index, exprs, into));
                    Self::gather_expr_used_expressions(*value, exprs, into);
                }
                Statement::Atomic { pointer, value, result, .. } => {
                    Self::gather_expr_used_expressions(*pointer, exprs, into);
                    Self::gather_expr_used_expressions(*value, exprs, into);
                    Self::gather_expr_used_expressions(*result, exprs, into);
                },
                Statement::Call { arguments, result, .. } => {
                    for arg in arguments {
                        Self::gather_expr_used_expressions(*arg, exprs, into);
                    }
                    result.map(|result| Self::gather_expr_used_expressions(result, exprs, into));
                },
                Statement::Break |
                Statement::Continue |
                Statement::Kill |
                Statement::Barrier(_) => (),
            }
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

        // gather used globals - we want to avoid importing globals / consts that are not required by the function,
        // but function local expressions contain all constants and global variables, whether used or not.
        let mut used_exprs = HashSet::new();
        Self::gather_block_used_expressions(&func.body, &func.expressions, &mut used_exprs);

        for (h_expr, expr) in func.expressions.iter() {
            let expr = match expr {
                Expression::CallResult(f) => Expression::CallResult(self.map_function_handle(f)),
                Expression::Constant(c) => {
                    if !used_exprs.contains(&h_expr) {
                        // emit a dummy expression
                        Expression::AtomicResult{ kind: naga::ScalarKind::Uint, width: 4, comparison: true }
                    } else {
                        Expression::Constant(self.import_const(c))
                    }
                },
                Expression::Compose { ty, components } => Expression::Compose {
                    ty: self.import_type(ty),
                    components: components.to_vec(),
                },
                Expression::GlobalVariable(gv) => {
                    if !used_exprs.contains(&h_expr) {
                        // emit a dummy expression
                        Expression::AtomicResult{ kind: naga::ScalarKind::Uint, width: 4, comparison: true }
                    } else {
                        Expression::GlobalVariable(self.import_global(gv))
                    }
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
                    image: *image,
                    sampler: *sampler,
                    gather: *gather,
                    coordinate: *coordinate,
                    array_index: *array_index,
                    offset: offset.map(|c| self.import_const(&c)),
                    level: *level,
                    depth_ref: *depth_ref,
                },
                // remaining expressions are bound to function context so don't need any modification
                Expression::Access { .. }
                | Expression::AccessIndex { .. }
                | Expression::Splat { .. }
                | Expression::Swizzle { .. }
                | Expression::FunctionArgument(_)
                | Expression::LocalVariable(_)
                | Expression::Load { .. }
                | Expression::ImageLoad { .. }
                | Expression::ImageQuery { .. }
                | Expression::Unary { .. }
                | Expression::Binary { .. }
                | Expression::Select { .. }
                | Expression::Derivative { .. }
                | Expression::Relational { .. }
                | Expression::Math { .. }
                | Expression::As { .. }
                | Expression::AtomicResult { .. }
                | Expression::ArrayLength(_) => expr.clone(),
            };
            let span = func.expressions.get_span(h_expr);
            expressions.append(expr, self.map_span(span));
        }

        let body = self.import_block(&func.body);

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
