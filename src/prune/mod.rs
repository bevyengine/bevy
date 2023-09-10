use naga::{
    Arena, Block, Constant, EntryPoint, Expression, Function, FunctionArgument, FunctionResult,
    GlobalVariable, Handle, ImageQuery, LocalVariable, Module, SampleLevel, Span, Statement,
    SwitchCase, Type, UniqueArena,
};
use std::collections::{hash_map::Entry, BTreeMap, HashMap, HashSet, VecDeque};
use tracing::{debug, trace};

use crate::derive::DerivedModule;
pub use naga::{Binding, BuiltIn};

#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
struct FunctionReq {
    body_required: BlockReq,
    exprs_required: HashMap<Handle<Expression>, PartReq>,
}

impl FunctionReq {
    fn prune(&self, func: &Function) -> Function {
        let arguments = func
            .arguments
            .iter()
            .map(|arg| FunctionArgument {
                name: arg.name.clone(),
                ty: arg.ty,
                binding: arg.binding.clone(),
            })
            .collect();

        let mut local_variables = Arena::new();
        let mut local_map = HashMap::new();
        for (h_l, l) in func.local_variables.iter() {
            if self.body_required.context.locals.contains_key(&h_l) {
                let new_local = LocalVariable {
                    name: l.name.clone(),
                    ty: l.ty,
                    init: l.init,
                };
                let new_h = local_variables.append(new_local, Span::UNDEFINED);
                local_map.insert(h_l, new_h);
            }
        }
        debug!("local map: {:?}", local_map);

        let mut expressions = Arena::new();
        let mut expr_map = HashMap::default();
        for (h_expr, expr) in func.expressions.iter() {
            if self.exprs_required.contains_key(&h_expr) {
                let new_h = expressions.append(
                    self.remap_expr(expr, &local_map, &expr_map),
                    Span::UNDEFINED,
                );
                expr_map.insert(h_expr, new_h);
            }
        }

        let mut named_expressions = indexmap::IndexMap::default();
        for (h_expr, name) in &func.named_expressions {
            if let Some(new_h) = expr_map.get(h_expr) {
                named_expressions.insert(*new_h, name.clone());
            }
        }

        let body = self.rewrite_block(&func.body, &self.body_required, &expr_map);
        let body = body.unwrap_or_default();

        let result = match self.body_required.context.retval {
            Some(_) => func.result.as_ref().map(|r| FunctionResult {
                ty: r.ty,
                binding: r.binding.clone(),
            }),
            None => None,
        };

        Function {
            name: func.name.clone(),
            arguments,
            result,
            local_variables,
            expressions,
            named_expressions,
            body,
        }
    }

    fn remap_expr(
        &self,
        expr: &Expression,
        local_map: &HashMap<Handle<LocalVariable>, Handle<LocalVariable>>,
        expr_map: &HashMap<Handle<Expression>, Handle<Expression>>,
    ) -> Expression {
        match expr {
            Expression::Literal(_) => expr.clone(),
            Expression::ZeroValue(_) => expr.clone(),
            Expression::WorkGroupUniformLoadResult { ty: _ty } => expr.clone(),
            Expression::Access { base, index } => Expression::Access {
                base: expr_map[base],
                index: expr_map[index],
            },
            Expression::AccessIndex { base, index } => Expression::AccessIndex {
                base: expr_map[base],
                index: *index,
            },
            Expression::Constant(c) => Expression::Constant(*c),
            Expression::Splat { size, value } => Expression::Splat {
                size: *size,
                value: expr_map[value],
            },
            Expression::Swizzle {
                size,
                vector,
                pattern,
            } => Expression::Swizzle {
                size: *size,
                vector: expr_map[vector],
                pattern: *pattern,
            },
            Expression::Compose { ty, components } => {
                let first_good = components
                    .iter()
                    .map(|c| expr_map.get(c))
                    .find(Option::is_some)
                    .unwrap()
                    .unwrap();
                Expression::Compose {
                    ty: *ty,
                    components: components
                        .iter()
                        .map(|c| expr_map.get(c).unwrap_or(first_good))
                        .copied()
                        .collect(),
                }
            }
            Expression::FunctionArgument(index) => Expression::FunctionArgument(*index),
            Expression::GlobalVariable(gv) => Expression::GlobalVariable(*gv),
            Expression::LocalVariable(lv) => Expression::LocalVariable(local_map[lv]),
            Expression::Load { pointer } => Expression::Load {
                pointer: expr_map[pointer],
            },
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
                image: expr_map[image],
                sampler: expr_map[sampler],
                gather: *gather,
                coordinate: expr_map[coordinate],
                array_index: array_index.map(|e| expr_map[&e]),
                offset: *offset,
                level: match level {
                    SampleLevel::Auto | SampleLevel::Zero => *level,
                    SampleLevel::Exact(e) => SampleLevel::Exact(expr_map[e]),
                    SampleLevel::Bias(e) => SampleLevel::Bias(expr_map[e]),
                    SampleLevel::Gradient { x, y } => SampleLevel::Gradient {
                        x: expr_map[x],
                        y: expr_map[y],
                    },
                },
                depth_ref: depth_ref.map(|e| expr_map[&e]),
            },
            Expression::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => Expression::ImageLoad {
                image: expr_map[image],
                coordinate: expr_map[coordinate],
                array_index: array_index.map(|e| expr_map[&e]),
                sample: sample.map(|e| expr_map[&e]),
                level: level.map(|e| expr_map[&e]),
            },
            Expression::ImageQuery { image, query } => Expression::ImageQuery {
                image: expr_map[image],
                query: match query {
                    ImageQuery::Size { level } => ImageQuery::Size {
                        level: level.map(|e| expr_map[&e]),
                    },
                    _ => *query,
                },
            },
            Expression::Unary { op, expr } => Expression::Unary {
                op: *op,
                expr: expr_map[expr],
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: expr_map[left],
                right: expr_map[right],
            },
            Expression::Select {
                condition,
                accept,
                reject,
            } => Expression::Select {
                condition: expr_map[condition],
                accept: expr_map[accept],
                reject: expr_map[reject],
            },
            Expression::Derivative { axis, expr, ctrl } => Expression::Derivative {
                axis: *axis,
                expr: expr_map[expr],
                ctrl: *ctrl,
            },
            Expression::Relational { fun, argument } => Expression::Relational {
                fun: *fun,
                argument: expr_map[argument],
            },
            Expression::Math {
                fun,
                arg,
                arg1,
                arg2,
                arg3,
            } => Expression::Math {
                fun: *fun,
                arg: expr_map[arg],
                arg1: arg1.map(|e| expr_map[&e]),
                arg2: arg2.map(|e| expr_map[&e]),
                arg3: arg3.map(|e| expr_map[&e]),
            },
            Expression::As {
                expr,
                kind,
                convert,
            } => Expression::As {
                expr: expr_map[expr],
                kind: *kind,
                convert: *convert,
            },
            Expression::CallResult(f) => Expression::CallResult(*f),
            Expression::AtomicResult { .. } => expr.clone(),
            Expression::ArrayLength(a) => Expression::ArrayLength(expr_map[a]),
            Expression::RayQueryProceedResult => expr.clone(),
            Expression::RayQueryGetIntersection { query, committed } => {
                Expression::RayQueryGetIntersection {
                    query: expr_map[query],
                    committed: *committed,
                }
            }
        }
    }

    fn rewrite_block(
        &self,
        block: &Block,
        block_req: &BlockReq,
        expr_map: &HashMap<Handle<Expression>, Handle<Expression>>,
    ) -> Option<Block> {
        let mut body = Vec::new();

        for (stmt, req) in block.iter().zip(block_req.required_statements.iter()) {
            if let Some(stmt) = self.rewrite_stmt(&block_req.context, stmt, req, expr_map) {
                body.push(stmt);
            }
        }

        if body.is_empty() {
            None
        } else {
            Some(Block::from_vec(body))
        }
    }

    fn rewrite_stmt(
        &self,
        context: &RequiredContext,
        stmt: &Statement,
        req: &StatementReq,
        expr_map: &HashMap<Handle<Expression>, Handle<Expression>>,
    ) -> Option<Statement> {
        match (stmt, req) {
            (Statement::Emit(es), StatementReq::Emit(bs)) => {
                let exprs: Vec<_> = es
                    .clone()
                    .zip(bs.iter())
                    .filter(|(_, b)| **b)
                    .map(|(e, _)| expr_map[&e])
                    .collect();

                if exprs.is_empty() {
                    return None;
                }

                let range =
                    naga::Range::new_from_bounds(*exprs.first().unwrap(), *exprs.last().unwrap());
                Some(Statement::Emit(range))
            }
            (Statement::Block(block), StatementReq::BlockStmt(reqs)) => self
                .rewrite_block(block, reqs, expr_map)
                .map(Statement::Block),
            (
                Statement::If {
                    condition,
                    accept,
                    reject,
                },
                StatementReq::If {
                    condition: condition_req,
                    accept: accept_req,
                    reject: reject_req,
                },
            ) => {
                if !condition_req {
                    return None;
                }

                let accept = self
                    .rewrite_block(accept, accept_req, expr_map)
                    .unwrap_or_default();
                let reject = self
                    .rewrite_block(reject, reject_req, expr_map)
                    .unwrap_or_default();
                Some(Statement::If {
                    condition: expr_map[condition],
                    accept,
                    reject,
                })
            }
            (
                Statement::Switch { selector, cases },
                StatementReq::Switch {
                    selector: selector_req,
                    cases: cases_req,
                },
            ) => {
                if !selector_req {
                    return None;
                }

                let cases = cases
                    .iter()
                    .zip(cases_req.iter())
                    .map(|(case, req)| SwitchCase {
                        value: case.value,
                        body: self
                            .rewrite_block(&case.body, req, expr_map)
                            .unwrap_or_default(),
                        fall_through: case.fall_through,
                    });

                Some(Statement::Switch {
                    selector: expr_map[selector],
                    cases: cases.collect(),
                })
            }
            (
                Statement::Loop {
                    body,
                    continuing,
                    break_if,
                },
                StatementReq::Loop {
                    body: body_req,
                    continuing: cont_req,
                    required: break_req,
                },
            ) => {
                if !break_req {
                    return None;
                }

                let body = self
                    .rewrite_block(body, body_req, expr_map)
                    .unwrap_or_default();
                let continuing = self
                    .rewrite_block(continuing, cont_req, expr_map)
                    .unwrap_or_default();

                Some(Statement::Loop {
                    body,
                    continuing,
                    break_if: break_if.map(|e| expr_map[&e]),
                })
            }
            (Statement::Break, _) => Some(Statement::Break),
            (Statement::Continue, _) => Some(Statement::Continue),
            (Statement::Return { value }, StatementReq::Return(b)) => {
                if !b {
                    return None;
                }

                if context.retval.is_some() {
                    Some(Statement::Return {
                        value: value.map(|e| expr_map[&e]),
                    })
                } else {
                    Some(Statement::Return { value: None })
                }
            }
            (Statement::Kill, _) => Some(Statement::Kill),
            (Statement::Barrier(b), _) => Some(Statement::Barrier(*b)),
            (Statement::Store { pointer, value }, StatementReq::Store(b)) => {
                if !b {
                    return None;
                }

                Some(Statement::Store {
                    pointer: expr_map[pointer],
                    value: expr_map[value],
                })
            }
            (
                Statement::ImageStore {
                    image,
                    coordinate,
                    array_index,
                    value,
                },
                StatementReq::ImageStore(b),
            ) => {
                if !b {
                    return None;
                }

                Some(Statement::ImageStore {
                    image: expr_map[image],
                    coordinate: expr_map[coordinate],
                    array_index: array_index.map(|e| expr_map[&e]),
                    value: expr_map[value],
                })
            }
            (
                Statement::Atomic {
                    pointer,
                    fun,
                    value,
                    result,
                },
                StatementReq::Atomic(b),
            ) => {
                if !b {
                    return None;
                }

                Some(Statement::Atomic {
                    pointer: expr_map[pointer],
                    fun: *fun,
                    value: expr_map[value],
                    result: expr_map[result],
                })
            }
            (
                Statement::Call {
                    function,
                    arguments,
                    result,
                },
                StatementReq::Call {
                    call_required,
                    result_required,
                },
            ) => {
                if !call_required {
                    return None;
                }

                let result = if *result_required {
                    result.map(|e| expr_map[&e])
                } else {
                    None
                };

                Some(Statement::Call {
                    function: *function,
                    arguments: arguments
                        .iter()
                        .map(|e| {
                            *expr_map
                                .get(e)
                                .unwrap_or_else(|| panic!("missing expr {:?}", e))
                        })
                        .collect(),
                    result,
                })
            }
            _ => panic!("unexpected pair {:?} + {:?}", stmt, req),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct BlockReq {
    required_statements: VecDeque<StatementReq>,
    context: RequiredContext,
}

impl BlockReq {
    fn is_required(&self) -> bool {
        self.required_statements.iter().any(StatementReq::required)
    }

    fn add(&self, other: &BlockReq) -> Self {
        use StatementReq::*;

        let required_statements = self
            .required_statements
            .iter()
            .zip(other.required_statements.iter())
            .map(|(s1, s2)| match (s1, s2) {
                (Emit(e1), Emit(e2)) => Emit(
                    e1.iter()
                        .zip(e2.iter())
                        .map(|(b1, b2)| *b1 || *b2)
                        .collect(),
                ),
                (BlockStmt(b1), BlockStmt(b2)) => BlockStmt(b1.add(b2)),
                (
                    If {
                        condition: c1,
                        accept: a1,
                        reject: r1,
                    },
                    If {
                        condition: c2,
                        accept: a2,
                        reject: r2,
                    },
                ) => If {
                    condition: *c1 || *c2,
                    accept: a1.add(a2),
                    reject: r1.add(r2),
                },
                (
                    Switch {
                        selector: s1,
                        cases: c1,
                    },
                    Switch {
                        selector: s2,
                        cases: c2,
                    },
                ) => Switch {
                    selector: *s1 || *s2,
                    cases: c1
                        .iter()
                        .zip(c2.iter())
                        .map(|(b1, b2)| b1.add(b2))
                        .collect(),
                },
                (
                    Loop {
                        body: b1,
                        continuing: c1,
                        required: bi1,
                    },
                    Loop {
                        body: b2,
                        continuing: c2,
                        required: bi2,
                    },
                ) => Loop {
                    body: b1.add(b2),
                    continuing: c1.add(c2),
                    required: *bi1 || *bi2,
                },
                (Break(r1), Break(r2)) => Break(*r1 || *r2),
                (Continue(r1), Continue(r2)) => Continue(*r1 || *r2),
                (Return(r1), Return(r2)) => Return(*r1 || *r2),
                (Kill(), Kill()) => Kill(),
                (Barrier(), Barrier()) => Barrier(),
                (Store(s1), Store(s2)) => Store(*s1 || *s2),
                (ImageStore(s1), ImageStore(s2)) => ImageStore(*s1 || *s2),
                (Atomic(a1), Atomic(a2)) => Atomic(*a1 || *a2),
                (
                    Call {
                        call_required: c1,
                        result_required: r1,
                    },
                    Call {
                        call_required: c2,
                        result_required: r2,
                    },
                ) => Call {
                    call_required: *c1 || *c2,
                    result_required: *r1 || *r2,
                },
                _ => panic!(),
            })
            .collect();

        let context = self.context.merge(&other.context);

        BlockReq {
            required_statements,
            context,
        }
    }
}

#[derive(Debug, Clone)]
enum StatementReq {
    Emit(Vec<bool>),
    BlockStmt(BlockReq),
    If {
        condition: bool,
        accept: BlockReq,
        reject: BlockReq,
    },
    Switch {
        selector: bool,
        cases: Vec<BlockReq>,
    },
    Loop {
        body: BlockReq,
        continuing: BlockReq,
        required: bool,
    },
    Break(bool),
    Continue(bool),
    Return(bool),
    Kill(),
    Barrier(),
    Store(bool),
    ImageStore(bool),
    Atomic(bool),
    Call {
        call_required: bool,
        result_required: bool,
    },
    RayQuery(bool),
}

impl StatementReq {
    fn required(&self) -> bool {
        match self {
            StatementReq::Emit(vr) => vr.iter().any(|r| *r),
            StatementReq::BlockStmt(b) => b.is_required(),
            StatementReq::If { condition, .. } => *condition,
            StatementReq::Switch { selector, .. } => *selector,
            StatementReq::Loop { required: break_if, .. } => *break_if,
            StatementReq::Barrier() => false, // this will be emitted but never makes a block required. todo: does this make sense? 
            StatementReq::Return(r) |    // return will be output if the block is output, but they should not make the block required unless we are within a required containing scope or the return value is required
                                            // this stops all functions appearing as required, even if retval is not required and no other part of the function is required
            StatementReq::Break(r) |
            StatementReq::Continue(r) => *r, // these will be output if the block is output, but they should not make the block required unless we are within a required containing scope
            StatementReq::Kill() => true,    // these always make the block required due to flow control
            StatementReq::Store(r) |
            StatementReq::ImageStore(r) |
            StatementReq::Atomic(r) => *r,
            StatementReq::Call{ call_required, .. } => *call_required,
            StatementReq::RayQuery(r) => *r,
        }
    }
}

// description of the required fraction of an expression or variable.
// after storing to the part, the requirement will be replaced by PartReq::Exist.
// required parts should not be removed (except by swizzle where it doesn't matter),
// only downgraded to Exist
#[derive(Clone, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum PartReq {
    All,
    Part(BTreeMap<usize, PartReq>),
    // needs to be present but contents don't matter
    Exist,
}

impl PartReq {
    fn get_type_bindings(&self, module: &Module, ty: Handle<Type>) -> HashSet<naga::Binding> {
        let ty = module.types.get_handle(ty).unwrap();

        match &ty.inner {
            naga::TypeInner::Struct { members, .. } => {
                let mut res = HashSet::default();
                for (i, member) in members.iter().enumerate() {
                    if let Some(part) = self.get_subpart(i) {
                        if !matches!(part, PartReq::Exist) {
                            if let Some(binding) = &member.binding {
                                res.insert(binding.clone());
                            }

                            res.extend(part.get_type_bindings(module, member.ty));
                        }
                    }
                }
                res
            }
            naga::TypeInner::Pointer { base, .. }
            | naga::TypeInner::Array { base, .. }
            | naga::TypeInner::BindingArray { base, .. } => self.get_type_bindings(module, *base),
            _ => Default::default(),
        }
    }

    fn from_type_bindings(
        module: &Module,
        required_bindings: &HashSet<naga::Binding>,
        ty: Handle<Type>,
    ) -> Option<Self> {
        trace!("from_type_bindings");

        let ty = module.types.get_handle(ty).unwrap();

        debug!("type: {:?}, required: {:?}", ty, required_bindings);

        match &ty.inner {
            naga::TypeInner::Struct { members, .. } => {
                let mut parts = BTreeMap::default();

                for (i, member) in members.iter().enumerate() {
                    if let Some(binding) = &member.binding {
                        if required_bindings.contains(binding) {
                            parts.insert(i, PartReq::All);
                        }
                    } else if let Some(part) =
                        Self::from_type_bindings(module, required_bindings, member.ty)
                    {
                        parts.insert(i, part);
                    }
                }

                if parts.is_empty() {
                    return None;
                }

                Some(PartReq::Part(parts))
            }
            _ => None,
        }
    }
}

impl PartReq {
    fn from_indexes(mut indexes: Vec<u32>) -> Self {
        if indexes.is_empty() {
            PartReq::All
        } else {
            let index = indexes.pop().unwrap();
            PartReq::Part(
                [(index as usize, Self::from_indexes(indexes))]
                    .into_iter()
                    .collect(),
            )
        }
    }

    fn get_subpart(&self, i: usize) -> Option<&PartReq> {
        match self {
            PartReq::All => Some(self),
            PartReq::Part(parts) => parts.get(&i),
            PartReq::Exist => None,
        }
    }

    fn contains(&self, other: &PartReq) -> bool {
        match (self, other) {
            (PartReq::All, _) => true,
            (_, PartReq::Exist) => true,
            (PartReq::Exist, _) => false,
            (PartReq::Part(_), PartReq::All) => false,

            (PartReq::Part(current), PartReq::Part(new)) => {
                return new.iter().all(|(index, other_subpart)| {
                    current.get(index).map_or(false, |current_subpart| {
                        current_subpart.contains(other_subpart)
                    })
                });
            }
        }
    }

    fn add(&self, other: &PartReq) -> PartReq {
        match (self, other) {
            (PartReq::All, _) | (_, PartReq::All) => PartReq::All,
            (PartReq::Exist, _) => other.clone(),
            (_, PartReq::Exist) => self.clone(),
            (PartReq::Part(a), PartReq::Part(b)) => {
                let mut merger = a.clone();

                for (index, other_subpart) in b.iter() {
                    if let Some(current_subpart) = merger.get_mut(index) {
                        *current_subpart = current_subpart.add(other_subpart);
                    } else {
                        merger.insert(*index, other_subpart.clone());
                    }
                }
                PartReq::Part(merger)
            }
        }
    }

    fn type_to_parts(
        ty: Handle<Type>,
        types: &UniqueArena<Type>,
    ) -> (PartReq, Option<Vec<Handle<Type>>>) {
        let ty = types.get_handle(ty).unwrap();
        match &ty.inner {
            naga::TypeInner::Vector { size, .. } => (PartReq::Part((0..*size as usize).map(|i| (i, PartReq::All)).collect()), None),
            naga::TypeInner::Matrix { columns, rows, .. } => {(
                PartReq::Part((0..*columns as usize).map(|c| (c, PartReq::Part((0..*rows as usize).map(|r| (r, PartReq::All)).collect()))).collect()),
                None
            )},
            naga::TypeInner::Struct { members, .. } => {(
                PartReq::Part((0..members.len()).map(|i| (i, PartReq::All)).collect()),
                Some(members.iter().map(|sm| sm.ty).collect())
            )},
            _ => (PartReq::All, None)
            // todo: we can probably do better for some of these ...
            // naga::TypeInner::Scalar { .. } => todo!(),
            // naga::TypeInner::Atomic { kind, width } => todo!(),
            // naga::TypeInner::Pointer { base, space } => todo!(),
            // naga::TypeInner::ValuePointer { size, kind, width, space } => todo!(),
            // naga::TypeInner::Array { base, size, stride } => todo!(),
            // naga::TypeInner::Image { dim, arrayed, class } => todo!(),
            // naga::TypeInner::Sampler { comparison } => todo!(),
            // naga::TypeInner::BindingArray { base, size } => todo!(),
        }
    }

    fn remove(
        &self,
        other: &PartReq,
        ty: Option<Handle<Type>>,
        types: &UniqueArena<Type>,
    ) -> PartReq {
        match other {
            PartReq::All => PartReq::Exist,
            PartReq::Exist => self.clone(),
            PartReq::Part(remove_parts) => {
                let (res, subtypes) = match (self, ty) {
                    (PartReq::All, Some(ty)) => Self::type_to_parts(ty, types),
                    (_, Some(ty)) => (self.clone(), Self::type_to_parts(ty, types).1),
                    (_, None) => (self.clone(), None),
                };

                if let PartReq::Part(current_parts) = res {
                    let current_parts = current_parts.into_iter().map(|(index, subpart)| {
                        if let Some(remove_subpart) = remove_parts.get(&index) {
                            let reduced = subpart.remove(
                                remove_subpart,
                                subtypes
                                    .as_ref()
                                    .map(|subtypes| *subtypes.get(index).unwrap()),
                                types,
                            );
                            (index, reduced)
                        } else {
                            (index, subpart)
                        }
                    });

                    PartReq::Part(current_parts.collect())
                } else {
                    res
                }
            }
        }
    }
}

// what we currently care about at a given point in the execution
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct RequiredContext {
    pub globals: HashMap<Handle<GlobalVariable>, PartReq>,
    pub locals: HashMap<Handle<LocalVariable>, PartReq>,
    pub args: Vec<Option<PartReq>>,
    pub retval: Option<PartReq>,
}

impl RequiredContext {
    pub fn globals_for_module(&self, module: &Module) {
        for (global, req) in &self.globals {
            let gv = module.global_variables.try_get(*global).unwrap();
            debug!("{}: {:?}", gv.name.as_ref().unwrap(), req);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.globals.is_empty()
            && self.locals.is_empty()
            && self.args.iter().all(Option::is_none)
            && self.retval.is_none()
    }

    /// map the required input context of the input module/function to create required output context for the output module/function.
    /// typically input module/function will be a fragment shader, and output module/function will be a vertex shader.
    pub fn input_to_bindings(
        &self,
        input_module: &Module,
        input_function: &Function,
    ) -> HashSet<Binding> {
        let mut required_bindings: HashSet<naga::Binding> = HashSet::default();

        for (arg_req, arg) in self.args.iter().zip(&input_function.arguments) {
            if let Some(arg_req) = arg_req {
                if matches!(arg_req, PartReq::Exist) {
                    continue;
                }

                if let Some(binding) = &arg.binding {
                    assert_eq!(arg_req, &PartReq::All);
                    required_bindings.insert(binding.clone());
                }

                required_bindings.extend(arg_req.get_type_bindings(input_module, arg.ty));
            }
        }

        required_bindings
    }

    pub fn output_from_bindings(
        required_bindings: &HashSet<Binding>,
        output_module: &Module,
        output_function: &Function,
    ) -> RequiredContext {
        let retval = output_function.result.as_ref().and_then(|result| {
            if let Some(binding) = &result.binding {
                if required_bindings.contains(binding) {
                    Some(PartReq::All)
                } else {
                    None
                }
            } else {
                PartReq::from_type_bindings(output_module, required_bindings, result.ty)
            }
        });

        RequiredContext {
            retval,
            ..Default::default()
        }
    }

    pub fn add_binding(
        &mut self,
        required_binding: Binding,
        output_module: &naga::Module,
        output_function: &Function,
    ) {
        let required_bindings = HashSet::from([required_binding]);

        let RequiredContext { retval, .. } =
            Self::output_from_bindings(&required_bindings, output_module, output_function);

        let final_retval = match (self.retval.take(), retval) {
            (Some(existing), Some(new)) => Some(existing.add(&new)),
            (Some(existing), None) => Some(existing),
            (None, new) => new,
        };

        self.retval = final_retval;
    }

    fn contains(&self, other: &RequiredContext) -> bool {
        if !other.globals.iter().all(|(gv, new_req)| {
            self.globals
                .get(gv)
                .map_or(false, |current_req| current_req.contains(new_req))
        }) {
            return false;
        }

        if !other.locals.iter().all(|(lv, new_req)| {
            self.locals
                .get(lv)
                .map_or(false, |current_req| current_req.contains(new_req))
        }) {
            return false;
        }

        for pair in self.args.iter().zip(other.args.iter()) {
            match pair {
                (None, Some(_)) => return false,
                (Some(current), Some(new)) if !current.contains(new) => return false,
                _ => (),
            }
        }

        if let Some(new_ret) = &other.retval {
            match &self.retval {
                None => return false,
                Some(cur_ret) if !cur_ret.contains(new_ret) => return false,
                _ => (),
            }
        }

        true
    }

    fn merge(&self, other: &RequiredContext) -> RequiredContext {
        let mut globals = self.globals.clone();
        for (gv, new_req) in &other.globals {
            if let Some(cur_req) = globals.get_mut(gv) {
                *cur_req = cur_req.add(new_req);
            } else {
                globals.insert(*gv, new_req.clone());
            }
        }

        let mut locals = self.locals.clone();
        for (lv, new_req) in &other.locals {
            if let Some(cur_req) = locals.get_mut(lv) {
                *cur_req = cur_req.add(new_req);
            } else {
                locals.insert(*lv, new_req.clone());
            }
        }

        let args = self
            .args
            .iter()
            .zip(other.args.iter())
            .map(|pair| match pair {
                (None, any_other) | (any_other, None) => any_other.clone(),
                (Some(arg1), Some(arg2)) => Some(arg1.add(arg2)),
            })
            .collect();

        let retval = match (&self.retval, &other.retval) {
            (None, any_other) | (any_other, None) => any_other.clone(),
            (Some(ret1), Some(ret2)) => Some(ret1.add(ret2)),
        };

        RequiredContext {
            globals,
            locals,
            args,
            retval,
        }
    }

    fn remove(&mut self, var: &VarRef, part: &PartReq, shader: &Module, function: &Function) {
        let remove_from = match var {
            VarRef::Global(gv) => self.globals.get_mut(gv).unwrap(),
            VarRef::Local(lv) => self.locals.get_mut(lv).unwrap(),
        };

        let ty = match var {
            VarRef::Global(gv) => shader.global_variables.try_get(*gv).unwrap().ty,
            VarRef::Local(lv) => function.local_variables.try_get(*lv).unwrap().ty,
        };

        *remove_from = remove_from.remove(part, Some(ty), &shader.types);
    }
}

#[derive(Debug)]
pub struct Pruner<'a> {
    module: &'a Module,
    types: HashSet<Handle<Type>>,
    entry_points: HashMap<String, FunctionReq>,
    functions: HashMap<Handle<Function>, FunctionReq>,
    globals: HashMap<Handle<GlobalVariable>, PartReq>,
    constants: HashSet<Handle<Constant>>,
}

#[derive(Debug)]
enum VarRef {
    Global(Handle<GlobalVariable>),
    Local(Handle<LocalVariable>),
}

#[derive(Debug)]
struct VarRefPart {
    var_ref: VarRef,
    part: PartReq,
}

impl<'a> Pruner<'a> {
    pub fn new(module: &'a Module) -> Self {
        Self {
            module,
            types: Default::default(),
            entry_points: Default::default(),
            functions: Default::default(),
            globals: Default::default(),
            constants: Default::default(),
        }
    }

    // returns what subpath of the var ref is required
    fn store_required(&self, context: &RequiredContext, var_ref: &VarRefPart) -> Option<PartReq> {
        let var_parts_required = match var_ref.var_ref {
            VarRef::Global(gv) => context.globals.get(&gv),
            VarRef::Local(lv) => context.locals.get(&lv),
        };

        debug!(
            "checking store requirement for {:?}; we need {:?}, and are targetting {:?}",
            var_ref.var_ref, var_parts_required, var_ref.part
        );

        fn check_part(required: Option<&PartReq>, targetted: &PartReq) -> Option<PartReq> {
            match (required, targetted) {
                (_, PartReq::Exist) | (None, _) | (Some(PartReq::Exist), _) => None,
                (Some(PartReq::All), _) => Some(PartReq::All),
                (Some(PartReq::Part(_)), PartReq::All) => required.cloned(), // assigning to the whole var, so we need what the var needs

                (Some(PartReq::Part(parts_required)), PartReq::Part(parts_assigned)) => {
                    assert!(parts_assigned.len() == 1);
                    let (part_assigned, subpart) = parts_assigned.iter().next().unwrap();

                    check_part(parts_required.get(part_assigned), subpart)
                }
            }
        }

        check_part(var_parts_required, &var_ref.part)
    }

    fn resolve_var(
        function: &Function,
        h_expr: Handle<Expression>,
        mut indexes: Vec<u32>,
    ) -> VarRefPart {
        let expr = function.expressions.try_get(h_expr).unwrap();
        match expr {
            Expression::Access { base, .. } => {
                // dynamic access force requires everything below it
                Self::resolve_var(function, *base, Vec::default())
            }
            Expression::AccessIndex { base, index } => {
                indexes.push(*index);
                Self::resolve_var(function, *base, indexes)
            }
            Expression::GlobalVariable(gv) => VarRefPart {
                var_ref: VarRef::Global(*gv),
                part: PartReq::from_indexes(indexes),
            },
            Expression::LocalVariable(lv) => VarRefPart {
                var_ref: VarRef::Local(*lv),
                part: PartReq::from_indexes(indexes),
            },
            _ => panic!("unexpected expr {:?} as var::pointer", expr),
        }
    }

    fn add_expression(
        &mut self,
        function: &Function,
        func_req: &mut FunctionReq,
        context: &mut RequiredContext,
        h_expr: Handle<Expression>,
        part: &PartReq,
    ) {
        let expr = function.expressions.try_get(h_expr).unwrap();

        debug!(
            "EXPR: adding {:?} of expr id {:?} - {:?}",
            part, h_expr, expr
        );

        match expr {
            Expression::Literal(_) => (),
            Expression::ZeroValue(_) => (),
            Expression::WorkGroupUniformLoadResult { .. } => (),
            Expression::AccessIndex { base, index } => self.add_expression(
                function,
                func_req,
                context,
                *base,
                &PartReq::Part([(*index as usize, part.clone())].into()),
            ),
            Expression::Access { base, index } => {
                self.add_expression(function, func_req, context, *base, &PartReq::All);
                self.add_expression(function, func_req, context, *index, &PartReq::All);
            }
            Expression::Constant(c) => {
                self.constants.insert(*c);
                assert!(part == &PartReq::All || part == &PartReq::Exist);
            }
            Expression::Splat { size: _size, value } => {
                self.add_expression(function, func_req, context, *value, &PartReq::All);
            }
            Expression::Swizzle {
                size: _size,
                vector,
                pattern,
            } => {
                let swizzled_req = match part {
                    PartReq::All => PartReq::All,
                    PartReq::Exist => PartReq::Exist,
                    PartReq::Part(parts) => {
                        // note - this doesn't honor PartReq::All -> PartReq::Exist for subparts, but since it can only operate on vectors it doesn't matter
                        let parts = parts.iter().map(|(index, req)| {
                            assert!(matches!(req, PartReq::All) || matches!(req, PartReq::Exist));
                            let component = pattern[*index];
                            (component as usize, req.clone())
                        });
                        PartReq::Part(parts.collect())
                    }
                };

                self.add_expression(function, func_req, context, *vector, &swizzled_req);
            }
            Expression::Compose {
                ty: _ty,
                components,
            } => match part {
                PartReq::All => {
                    for component in components {
                        self.add_expression(function, func_req, context, *component, &PartReq::All);
                    }
                }
                PartReq::Part(parts) => {
                    for (index, subreq) in parts {
                        let component = components[*index];
                        self.add_expression(function, func_req, context, component, subreq);
                    }
                }
                PartReq::Exist => (),
            },
            Expression::FunctionArgument(index) => {
                let current = &context.args[*index as usize];
                let new = match current {
                    Some(cur) => cur.add(part),
                    None => part.clone(),
                };
                context.args[*index as usize] = Some(new);
            }
            Expression::GlobalVariable(gv) => {
                let entry = self.globals.entry(*gv);
                match entry {
                    Entry::Occupied(mut cur) => *cur.get_mut() = cur.get().add(part),
                    Entry::Vacant(_) => {
                        let ty = self.module.global_variables.try_get(*gv).unwrap().ty;
                        self.types.insert(ty);
                        self.globals.insert(*gv, part.clone());
                        context.globals.insert(*gv, part.clone());
                    }
                }
            }
            Expression::LocalVariable(lv) => {
                let entry = context.locals.entry(*lv);
                match entry {
                    Entry::Occupied(mut cur) => *cur.get_mut() = cur.get().add(part),
                    Entry::Vacant(_) => {
                        let ty = function.local_variables.try_get(*lv).unwrap().ty;
                        self.types.insert(ty);
                        context.locals.insert(*lv, part.clone());
                    }
                }
            }
            Expression::Load { pointer } => {
                self.add_expression(function, func_req, context, *pointer, part);
            }
            Expression::ImageSample {
                image,
                sampler,
                gather: _gather,
                coordinate,
                array_index,
                offset: _offset,
                level,
                depth_ref,
            } => {
                self.add_expression(function, func_req, context, *image, &PartReq::All);
                self.add_expression(function, func_req, context, *sampler, &PartReq::All);
                self.add_expression(function, func_req, context, *coordinate, &PartReq::All);
                array_index
                    .map(|e| self.add_expression(function, func_req, context, e, &PartReq::All));
                match level {
                    naga::SampleLevel::Auto | naga::SampleLevel::Zero => (),
                    naga::SampleLevel::Exact(e) | naga::SampleLevel::Bias(e) => {
                        self.add_expression(function, func_req, context, *e, &PartReq::All);
                    }
                    naga::SampleLevel::Gradient { x, y } => {
                        self.add_expression(function, func_req, context, *x, &PartReq::All);
                        self.add_expression(function, func_req, context, *y, &PartReq::All);
                    }
                };
                depth_ref
                    .map(|e| self.add_expression(function, func_req, context, e, &PartReq::All));
            }
            Expression::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => {
                self.add_expression(function, func_req, context, *image, &PartReq::All);
                self.add_expression(function, func_req, context, *coordinate, &PartReq::All);
                array_index
                    .map(|e| self.add_expression(function, func_req, context, e, &PartReq::All));
                sample.map(|e| self.add_expression(function, func_req, context, e, &PartReq::All));
                level.map(|e| self.add_expression(function, func_req, context, e, &PartReq::All));
            }
            Expression::ImageQuery { image, query } => {
                self.add_expression(function, func_req, context, *image, &PartReq::All);
                if let ImageQuery::Size { level: Some(level) } = query {
                    self.add_expression(function, func_req, context, *level, &PartReq::All);
                }
            }
            Expression::Unary { op: _op, expr } => {
                self.add_expression(function, func_req, context, *expr, part);
            }
            Expression::Binary {
                op: _op,
                left,
                right,
            } => {
                self.add_expression(function, func_req, context, *left, part);
                self.add_expression(function, func_req, context, *right, part);
            }
            Expression::Select {
                condition,
                accept,
                reject,
            } => {
                self.add_expression(function, func_req, context, *condition, &PartReq::All);
                self.add_expression(function, func_req, context, *accept, part);
                self.add_expression(function, func_req, context, *reject, part);
            }
            Expression::Derivative { expr, .. } => {
                self.add_expression(function, func_req, context, *expr, &PartReq::All);
            }
            Expression::Relational {
                fun: _fun,
                argument,
            } => {
                self.add_expression(function, func_req, context, *argument, &PartReq::All);
            }
            Expression::Math {
                fun: _fun,
                arg,
                arg1,
                arg2,
                arg3,
            } => {
                self.add_expression(function, func_req, context, *arg, &PartReq::All);
                for arg in [arg1, arg2, arg3].into_iter().flatten() {
                    self.add_expression(function, func_req, context, *arg, &PartReq::All);
                }
            }
            Expression::As {
                expr,
                kind: _kind,
                convert: _convert,
            } => {
                self.add_expression(function, func_req, context, *expr, part);
            }
            Expression::CallResult(_f) => {
                // nothing, handled by the statement
            }
            Expression::AtomicResult { .. } => {
                // nothing, handled by the statement
            }
            Expression::ArrayLength(expr) => {
                let part = PartReq::Exist;
                self.add_expression(function, func_req, context, *expr, &part);
            }
            Expression::RayQueryProceedResult => (),
            Expression::RayQueryGetIntersection {
                query,
                committed: _committed,
            } => {
                self.add_expression(function, func_req, context, *query, &PartReq::All);
            }
        }

        func_req.exprs_required.insert(h_expr, part.clone());
    }

    fn add_statement(
        &mut self,
        function: &Function,
        func_req: &mut FunctionReq,
        context: &mut RequiredContext,
        stmt: &Statement,
        break_required: bool,
        break_context: &RequiredContext,
    ) -> StatementReq {
        use StatementReq::*;

        debug!("STATEMENT: parsing {:?}", stmt);

        match stmt {
            Statement::Emit(rng) => {
                let reqs = rng
                    .clone()
                    .map(|expr| func_req.exprs_required.contains_key(&expr))
                    .collect();
                Emit(reqs)
            }
            Statement::Block(b) => {
                let block = self.add_block(function, func_req, context, b, break_required);
                *context = block.context.clone();
                BlockStmt(block)
            }
            Statement::If {
                condition,
                accept,
                reject,
            } => {
                let accept_req =
                    self.add_block(function, func_req, context, accept, break_required);
                let reject_req =
                    self.add_block(function, func_req, context, reject, break_required);
                let condition_req = accept_req.is_required() || reject_req.is_required();

                debug!(
                    "if required? {} (break required is {})",
                    condition_req, break_required
                );

                debug!(
                    "reject: {:?} : {:?}, required: {}",
                    reject,
                    reject_req,
                    reject_req.is_required()
                );

                if condition_req {
                    *context = accept_req.context.merge(&reject_req.context);
                    self.add_expression(function, func_req, context, *condition, &PartReq::All);
                }

                If {
                    condition: condition_req,
                    accept: accept_req,
                    reject: reject_req,
                }
            }
            Statement::Switch { selector, cases } => {
                let mut any_req = false;
                let mut reqs = Vec::new();

                let mut input_context = context.clone();
                for case in cases.iter().rev() {
                    let case = self.add_block(function, func_req, context, &case.body, false);
                    input_context = input_context.merge(&case.context);
                    any_req |= case.is_required();
                    reqs.push(case);
                }

                if any_req {
                    self.add_expression(function, func_req, context, *selector, &PartReq::All);
                    *context = input_context;
                }

                Switch {
                    selector: any_req,
                    cases: reqs,
                }
            }
            Statement::Loop {
                body: body_in,
                continuing: cont_in,
                break_if,
            } => {
                debug!("loop first pass");
                let mut body = self.add_block(function, func_req, context, body_in, false);
                let mut continuing = self.add_block(function, func_req, context, cont_in, false);
                let loop_required = body.is_required() || continuing.is_required();

                debug!("loop required? {}", loop_required);
                if loop_required {
                    if let Some(break_if) = break_if {
                        self.add_expression(function, func_req, context, *break_if, &PartReq::All);
                    }

                    let working_context = body.context.merge(&continuing.context);

                    loop {
                        // rerun after adding break condition (else it may think the condition is not required in the blocks)
                        body = self.add_block(function, func_req, &working_context, body_in, true);
                        continuing =
                            self.add_block(function, func_req, &working_context, cont_in, true);

                        let new_context = body.context.merge(&continuing.context);
                        if working_context.contains(&new_context) {
                            break;
                        }
                    }

                    *context = working_context;
                }

                Loop {
                    body,
                    continuing,
                    required: loop_required,
                }
            }
            Statement::Break => {
                debug!("adding break({})", break_required);
                *context = break_context.clone();
                Break(break_required)
            }
            Statement::Continue => {
                *context = break_context.clone();
                Continue(break_required)
            }
            Statement::Return { value } => {
                let part = context.retval.clone();
                if let Some(value) = value {
                    debug!("return part: {:?} of {:?}", part, value);
                    if let Some(part) = part.as_ref() {
                        self.add_expression(function, func_req, context, *value, part);
                        return Return(true);
                    }
                }
                *context = break_context.clone();
                Return(break_required)
            }
            Statement::Kill => Kill(),
            Statement::Barrier(_) => Barrier(),
            Statement::Store { pointer, value } => {
                let var_ref = Self::resolve_var(function, *pointer, Vec::default());
                let required = self.store_required(context, &var_ref);

                debug!("store required from var: {:?}", required);

                match required {
                    Some(part_req @ PartReq::All) | Some(part_req @ PartReq::Part(_)) => {
                        // we no longer care about what writes to this variable
                        debug!("context prior to removal: {:?}", context);
                        debug!("removing {:?} from {:?}", var_ref.part, var_ref.var_ref);
                        context.remove(&var_ref.var_ref, &var_ref.part, self.module, function);
                        debug!("context after to removal: {:?}", context);

                        // ensure the path to the variable exists
                        self.add_expression(function, func_req, context, *pointer, &PartReq::Exist);

                        // and the needed part of the stored value
                        self.add_expression(function, func_req, context, *value, &part_req);

                        Store(true)
                    }
                    _ => Store(false),
                }
            }
            Statement::Atomic {
                pointer,
                fun: _fun,
                value,
                result,
            } => {
                let var_ref = Self::resolve_var(function, *pointer, Vec::default());
                let required_store = self.store_required(context, &var_ref);
                debug!("atomic store required: {:?}", required_store);

                let required_load = func_req.exprs_required.get(result).cloned();
                debug!("atomic load required: {:?}", required_load);

                if required_load.is_some() || required_store.is_some() {
                    // just pass it all through i guess ..?
                    if let Some(required_load) = required_load {
                        self.add_expression(function, func_req, context, *result, &required_load);
                    }
                    if let Some(required_store) = required_store {
                        self.add_expression(function, func_req, context, *value, &required_store);
                    }

                    Atomic(true)
                } else {
                    Atomic(false)
                }
            }
            Statement::Call {
                function: call_func,
                arguments,
                result,
            } => {
                let mut req = None;

                if let Some(result) = result {
                    if let Some(part) = func_req.exprs_required.get(result) {
                        req = Some(part.clone());
                    }
                }

                let (func_required, input_context) =
                    self.add_function(*call_func, context.globals.clone(), req.clone());

                if func_required {
                    debug!(
                        "adding args for required func: {:?} / {:?}",
                        arguments, input_context.args
                    );
                    for (arg, req) in arguments.iter().zip(input_context.args.iter()) {
                        if let Some(req) = req {
                            self.add_expression(function, func_req, context, *arg, req);
                        }
                    }

                    let mut result_required = false;
                    if let Some(result) = result {
                        if let Some(req) = req {
                            self.add_expression(function, func_req, context, *result, &req);
                            result_required = true;
                        }
                    }

                    // need to check if func is required somehow.
                    // it can modify a global we rely on that doesn't rely on inputs or outputs.
                    StatementReq::Call {
                        call_required: true,
                        result_required,
                    }
                } else {
                    StatementReq::Call {
                        call_required: false,
                        result_required: false,
                    }
                }
            }
            Statement::ImageStore {
                image,
                coordinate,
                array_index,
                value,
            } => {
                let var_ref = Self::resolve_var(function, *image, Vec::default());
                let required = self.store_required(context, &var_ref);

                debug!("imgstore required from var: {:?}", required);

                match required {
                    Some(part_req @ PartReq::All) | Some(part_req @ PartReq::Part(_)) => {
                        // we no longer care about what writes to this variable
                        debug!("context prior to removal: {:?}", context);
                        debug!("removing {:?} from {:?}", var_ref.part, var_ref.var_ref);
                        context.remove(&var_ref.var_ref, &var_ref.part, self.module, function);
                        debug!("context after to removal: {:?}", context);

                        // ensure the path to the variable exists
                        self.add_expression(function, func_req, context, *image, &PartReq::Exist);

                        // all of the accessors
                        self.add_expression(
                            function,
                            func_req,
                            context,
                            *coordinate,
                            &PartReq::All,
                        );
                        if let Some(ix) = array_index {
                            self.add_expression(function, func_req, context, *ix, &PartReq::Exist);
                        }

                        // and the needed part of the stored value
                        self.add_expression(function, func_req, context, *value, &part_req);

                        ImageStore(true)
                    }
                    _ => ImageStore(false),
                }
            }
            Statement::RayQuery { query, fun: _fun } => {
                let var_ref = Self::resolve_var(function, *query, Vec::default());
                let required = self.store_required(context, &var_ref);
                RayQuery(required.is_some())
            }
            Statement::WorkGroupUniformLoad { pointer, result } => {
                let var_ref = Self::resolve_var(function, *result, Vec::default());
                let required = self.store_required(context, &var_ref).is_some();
                if required {
                    self.add_expression(function, func_req, context, *pointer, &PartReq::All);
                }
                RayQuery(required)
            }
        }
    }

    fn add_block(
        &mut self,
        function: &Function,
        func_req: &mut FunctionReq,
        base_context: &RequiredContext,
        block: &Block,
        break_required: bool,
    ) -> BlockReq {
        trace!("BLOCK BEGIN");
        let mut blockreq = BlockReq {
            context: base_context.clone(),
            ..Default::default()
        };
        let mut break_required = break_required;

        for stmt in block.iter().rev() {
            let req = self.add_statement(
                function,
                func_req,
                &mut blockreq.context,
                stmt,
                break_required,
                base_context,
            );
            break_required |= req.required();
            blockreq.required_statements.push_front(req);

            debug!("context: {:?}", blockreq.context);
        }

        trace!("BLOCK END");
        debug!("func_req.body: {:?}", func_req.body_required);
        blockreq
    }

    fn add_function_ref(
        &mut self,
        func: &Function,
        globals: HashMap<Handle<GlobalVariable>, PartReq>,
        retval: Option<PartReq>,
    ) -> FunctionReq {
        let context = RequiredContext {
            globals,
            retval,
            locals: Default::default(),
            args: vec![None; func.arguments.len()],
        };

        debug!("> func ref : {:?}", func.name);
        debug!("req context: {:?}", context);

        let mut func_req = FunctionReq {
            body_required: Default::default(),
            exprs_required: Default::default(),
        };

        let block = &func.body;

        let new_block = self.add_block(func, &mut func_req, &context, block, false);
        func_req.body_required = new_block;

        debug!("< func ref : {:?}", func.name);
        func_req
    }

    pub fn add_function(
        &mut self,
        function: Handle<Function>,
        globals: HashMap<Handle<GlobalVariable>, PartReq>,
        retval: Option<PartReq>,
    ) -> (bool, RequiredContext) {
        debug!("> function: {:?}", function);

        let func = self.module.functions.try_get(function).unwrap();
        let func_req = self.add_function_ref(func, globals, retval);
        let required = func_req.body_required.is_required();
        let context = func_req.body_required.context.clone();

        match self.functions.get_mut(&function) {
            Some(f) => {
                f.body_required = f.body_required.add(&func_req.body_required);
                f.exprs_required.extend(func_req.exprs_required);
            }
            None => {
                self.functions.insert(function, func_req);
            }
        };

        // self.func_io_cache.insert((function, required_output.clone()), required_input.clone());
        debug!("< function: {:?}", function);
        debug!("req: {}, input context: {:?}", required, context);
        (required, context)
    }

    pub fn add_entrypoint(
        &mut self,
        entrypoint: &EntryPoint,
        globals: HashMap<Handle<GlobalVariable>, PartReq>,
        retval: Option<PartReq>,
    ) -> RequiredContext {
        let func_req = self.add_function_ref(&entrypoint.function, globals, retval);
        debug!("< entry_point: {}", entrypoint.name);
        debug!("input context: {:?}", func_req.body_required.context);

        let context = func_req.body_required.context.clone();

        match self.entry_points.get_mut(&entrypoint.name) {
            Some(f) => {
                f.body_required = f.body_required.add(&func_req.body_required);
                f.exprs_required.extend(func_req.exprs_required);
            }
            None => {
                self.entry_points.insert(entrypoint.name.clone(), func_req);
            }
        };

        context
    }

    pub fn rewrite(&self) -> Module {
        let mut derived = DerivedModule::default();
        derived.set_shader_source(self.module, 0);

        // just copy all the constants for now, so we can copy const handles as well
        for (h_cexpr, _) in self.module.const_expressions.iter() {
            derived.import_const_expression(h_cexpr);
        }

        for (h_f, f) in self.module.functions.iter() {
            if let Some(req) = self.functions.get(&h_f) {
                if req.body_required.is_required() {
                    trace!("rewrite {:?}", f.name);
                    debug!("func req: {:#?}", req);
                    let new_f = req.prune(f);
                    derived.import_function(&new_f, Span::UNDEFINED);
                    debug!("map {:?} -> {:?}", h_f, derived.map_function_handle(&h_f));
                }
            }
        }

        let mut entry_points = Vec::new();
        for ep in &self.module.entry_points {
            if let Some(req) = self.entry_points.get(&ep.name) {
                trace!("rewrite {}", ep.name);
                debug!("func req: {:#?}", req);

                let new_f = req.prune(&ep.function);
                let mapped_f = derived.localize_function(&new_f);
                let new_ep = EntryPoint {
                    name: ep.name.clone(),
                    stage: ep.stage,
                    early_depth_test: ep.early_depth_test,
                    workgroup_size: ep.workgroup_size,
                    function: mapped_f,
                };
                entry_points.push(new_ep);
            }
        }

        fn count_stmts(block: &Block) -> usize {
            let mut count = 0;
            for stmt in block.iter() {
                count += match stmt {
                    Statement::Block(b) => count_stmts(b),
                    Statement::If { accept, reject, .. } => {
                        count_stmts(accept) + count_stmts(reject)
                    }
                    Statement::Switch { cases, .. } => cases
                        .iter()
                        .map(|case| count_stmts(&case.body))
                        .sum::<usize>(),
                    Statement::Loop {
                        body, continuing, ..
                    } => count_stmts(body) + count_stmts(continuing),
                    _ => 1,
                }
            }

            count
        }

        let pruned = Module {
            entry_points,
            ..derived.into()
        };

        let exprs_now = pruned
            .functions
            .iter()
            .map(|(_, f)| f.expressions.len())
            .sum::<usize>()
            + pruned
                .entry_points
                .iter()
                .map(|e| e.function.expressions.len())
                .sum::<usize>();
        let exprs_before = self
            .module
            .functions
            .iter()
            .map(|(_, f)| f.expressions.len())
            .sum::<usize>()
            + self
                .module
                .entry_points
                .iter()
                .map(|e| e.function.expressions.len())
                .sum::<usize>();
        let stmts_now = pruned
            .functions
            .iter()
            .map(|(_, f)| count_stmts(&f.body))
            .sum::<usize>()
            + pruned
                .entry_points
                .iter()
                .map(|e| count_stmts(&e.function.body))
                .sum::<usize>();
        let stmts_before = self
            .module
            .functions
            .iter()
            .map(|(_, f)| count_stmts(&f.body))
            .sum::<usize>()
            + self
                .module
                .entry_points
                .iter()
                .map(|e| count_stmts(&e.function.body))
                .sum::<usize>();

        debug!(
            "[ty: {}/{}, const: {}/{}, globals: {}/{}, funcs: {}/{}, exprs: {}/{}, stmts: {}/{}]",
            pruned.types.len(),
            self.module.types.len(),
            pruned.constants.len(),
            self.module.constants.len(),
            pruned.global_variables.len(),
            self.module.global_variables.len(),
            pruned.functions.len(),
            self.module.functions.len(),
            exprs_now,
            exprs_before,
            stmts_now,
            stmts_before,
        );

        pruned
    }
}
