//! Constraint system for component relationships.
//!
//! Provides four primitives (`Required`, `Not`, `And`, `Or`) that can express
//! any boolean constraint over component sets.

use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, fmt};
use fixedbitset::FixedBitSet;

use super::ComponentId;

/// [`ComponentConstraint`] stored in `ComponentInfo`
#[derive(Debug, Clone)]
pub struct ComponentConstraint {
    /// Compiled DNF form
    pub dnf: Option<Dnf>,

    /// [`ComponentId`] set
    pub only: Option<FixedBitSet>,
}

impl ComponentConstraint {
    /// build from expr
    pub fn from_expr(expr: Option<ConstraintExpr>, only: Option<Vec<ComponentId>>) -> Self {
        ComponentConstraint {
            dnf: expr.map(|e| e.to_dnf()),
            only: only.map(|ids| {
                let max = ids.iter().map(|id| id.index()).max().unwrap_or(0);
                let mut bits = FixedBitSet::with_capacity(max + 1);
                for id in &ids {
                    bits.insert(id.index());
                }
                bits
            }),
        }
    }
}

/// Error returned when a component constraint is violated during archetype creation.
#[derive(Debug)]
pub struct ComponentsConstraintError {
    /// The component whose constraint was violated.
    pub component: ComponentId,
    /// Components that were missing. Each entry is one `required` field in [`DnfClause`]
    pub missing: Vec<Vec<ComponentId>>,
    /// Components that were present but forbidden. Each entry is one `forbidden` field in [`DnfClause`]
    pub conflicting: Vec<Vec<ComponentId>>,
    /// Components that were present but been disallowed(not in `only`)
    pub disallowed: Vec<ComponentId>,
}

impl fmt::Display for ComponentsConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Constraint violated for component {:?}", self.component)?;
        if !self.missing.is_empty() {
            write!(f, ", missing: {:?}", self.missing)?;
        }
        if !self.conflicting.is_empty() {
            write!(f, ", conflicts with: {:?}", self.conflicting)?;
        }
        if !self.disallowed.is_empty() {
            write!(
                f,
                ", disallowed with {:?}(not in \"only\" field)",
                self.disallowed
            )?;
        }
        Ok(())
    }
}

// Currently we use `core::error::Error` since there is only error propagation path from `get_id_or_insert` in [`Archetype`] impl
impl Error for ComponentsConstraintError {}

/// A constraint expression over component presence
///
/// Constraints are attached to a component and express: "if I exist in an archetype,
/// then this predicate must hold over that archetype's component set."
#[derive(Debug, Clone)]
pub enum ConstraintExpr {
    /// The given component must be present.
    Required(ComponentId),
    /// Negates the inner constraint.
    Not(Box<ConstraintExpr>),
    /// All inner constraints must hold.
    And(Vec<ConstraintExpr>),
    /// At least one inner constraint must hold.
    Or(Vec<ConstraintExpr>),
}

/// Primitive: "I need this component in current archetype."
pub fn require(id: ComponentId) -> ConstraintExpr {
    ConstraintExpr::Required(id)
}

/// The component must NOT be present. Sugar for `not(require(id))`.
pub fn forbid(id: ComponentId) -> ConstraintExpr {
    ConstraintExpr::Not(Box::new(ConstraintExpr::Required(id)))
}

/// Negates a constraint
pub fn not(constraint: ConstraintExpr) -> ConstraintExpr {
    ConstraintExpr::Not(Box::new(constraint))
}

/// All constraints must hold
pub fn and(constraints: impl Into<Vec<ConstraintExpr>>) -> ConstraintExpr {
    ConstraintExpr::And(constraints.into())
}

/// At least one constraint must hold
pub fn or(constraints: impl Into<Vec<ConstraintExpr>>) -> ConstraintExpr {
    ConstraintExpr::Or(constraints.into())
}

impl ConstraintExpr {
    /// Convert this constraint into Disjunctive Normal Form for efficient evaluation.
    pub(super) fn to_dnf(&self) -> Dnf {
        let clauses = to_dnf_clauses(self);
        Dnf { clauses }
    }
}

/// A single clause in DNF form: a conjunction of positive and negative literals.
#[derive(Debug, Clone)]
pub struct DnfClause {
    /// Components that must be present.
    pub required: FixedBitSet,
    /// Components that must NOT be present.
    pub forbidden: FixedBitSet,
}

impl DnfClause {
    fn new() -> Self {
        Self {
            required: FixedBitSet::new(),
            forbidden: FixedBitSet::new(),
        }
    }

    /// Returns `true` if this clause is satisfiable (required and forbidden don't overlap).
    fn is_satisfiable(&self) -> bool {
        self.required.intersection(&self.forbidden).count() == 0
    }

    /// Check if this clause is satisfied by the given archetype component bitset.
    fn satisfied_by(&self, archetype_bits: &FixedBitSet) -> bool {
        self.required.is_subset(archetype_bits)
            && self.forbidden.intersection(archetype_bits).count() == 0
    }

    /// Merge another clause into this one (AND semantics: union both sets).
    fn merge(&self, other: &DnfClause) -> DnfClause {
        let mut required = self.required.clone();
        let mut forbidden = self.forbidden.clone();
        // Grow to fit if needed
        let max_req = other.required.len().max(required.len());
        let max_forb = other.forbidden.len().max(forbidden.len());
        required.grow(max_req);
        forbidden.grow(max_forb);
        required.union_with(&other.required);
        forbidden.union_with(&other.forbidden);
        DnfClause {
            required,
            forbidden,
        }
    }
}

/// A constraint in Disjunctive Normal Form: a disjunction (OR) of conjunctive clauses.
#[derive(Debug, Clone)]
pub struct Dnf {
    clauses: Vec<DnfClause>,
}

impl Dnf {
    /// Empty
    pub fn empty() -> Self {
        Self {
            clauses: Vec::new(),
        }
    }

    /// A DNF that is always satisfied (True).
    pub fn tautology() -> Self {
        Self {
            clauses: alloc::vec![DnfClause::new()],
        }
    }

    /// Check if this DNF is satisfied by the given archetype component bitset.
    pub fn satisfied_by(&self, archetype_bits: &FixedBitSet) -> bool {
        self.clauses
            .iter()
            .any(|clause| clause.satisfied_by(archetype_bits))
    }

    /// Returns the clauses of this DNF.
    pub fn clauses(&self) -> &[DnfClause] {
        &self.clauses
    }
}

/// Convert a [`ConstraintExpr`] tree into a list of [`DnfClause`]s.
fn to_dnf_clauses(constraint: &ConstraintExpr) -> Vec<DnfClause> {
    match constraint {
        ConstraintExpr::Required(id) => {
            let mut clause = DnfClause::new();
            let idx = id.index();
            clause.required.grow(idx + 1);
            clause.required.insert(idx);
            alloc::vec![clause]
        }
        ConstraintExpr::Not(inner) => negate_dnf(&to_dnf_clauses(inner)),
        ConstraintExpr::And(children) => {
            let mut result = alloc::vec![DnfClause::new()]; // single empty clause = true
            for child in children {
                let child_clauses = to_dnf_clauses(child);
                result = and_dnf(result, child_clauses);
            }
            result
        }
        ConstraintExpr::Or(children) => {
            let mut result = Vec::new();
            for child in children {
                result.extend(to_dnf_clauses(child));
            }
            // Remove unsatisfiable clauses
            result.retain(DnfClause::is_satisfiable);
            result
        }
    }
}

/// AND two DNFs: distribute (cross-product of clauses, merging each pair).
fn and_dnf(left: Vec<DnfClause>, right: Vec<DnfClause>) -> Vec<DnfClause> {
    let mut result = Vec::with_capacity(left.len() * right.len());
    for l in &left {
        for r in &right {
            let merged = l.merge(r);
            if merged.is_satisfiable() {
                result.push(merged);
            }
        }
    }
    result
}

/// Negate a DNF. NOT(OR(c1, c2, ...)) = AND(NOT(c1), NOT(c2), ...).
/// Each clause negation: NOT(AND(required, NOT(forbidden))) uses De Morgan's law.
fn negate_dnf(clauses: &[DnfClause]) -> Vec<DnfClause> {
    // Negate each clause into a small DNF, then AND them all together.
    let mut result = alloc::vec![DnfClause::new()]; // tautology
    for clause in clauses {
        let negated = negate_clause(clause);
        result = and_dnf(result, negated);
    }
    result
}

/// Negate a single conjunctive clause.
/// NOT(a AND b AND NOT c AND NOT d) = (NOT a) OR (NOT b) OR c OR d
fn negate_clause(clause: &DnfClause) -> Vec<DnfClause> {
    let mut result = Vec::new();
    // For each required bit, create a clause that forbids it
    for idx in clause.required.ones() {
        let mut c = DnfClause::new();
        c.forbidden.grow(idx + 1);
        c.forbidden.insert(idx);
        result.push(c);
    }
    // For each forbidden bit, create a clause that requires it
    for idx in clause.forbidden.ones() {
        let mut c = DnfClause::new();
        c.required.grow(idx + 1);
        c.required.insert(idx);
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(ids: &[usize]) -> FixedBitSet {
        let max = ids.iter().copied().max().unwrap_or(0);
        let mut bs = FixedBitSet::with_capacity(max + 1);
        for &id in ids {
            bs.insert(id);
        }
        bs
    }

    fn cid(index: usize) -> ComponentId {
        ComponentId::new(index)
    }

    #[test]
    fn mutual_exclusion() {
        // Player and Enemy cannot coexist
        let c = forbid(cid(2)); // forbid Enemy(2)
        let dnf = c.to_dnf();
        assert!(dnf.satisfied_by(&bits(&[1]))); // Player only
        assert!(!dnf.satisfied_by(&bits(&[1, 2]))); // Player + Enemy
    }

    #[test]
    fn conditional_dependency() {
        // if Mana(3) exists then Caster(4) must exist
        // or(not(Mana), required(Caster))
        let c = or([forbid(cid(3)), require(cid(4))]);
        let dnf = c.to_dnf();
        assert!(dnf.satisfied_by(&bits(&[1, 2]))); // no Mana, no Caster: ok
        assert!(dnf.satisfied_by(&bits(&[3, 4]))); // Mana + Caster: ok
        assert!(!dnf.satisfied_by(&bits(&[3]))); // Mana without Caster: fail
    }

    #[test]
    fn contradiction_detected() {
        // A requires B, B requires C, C forbids A
        let c_constraint = forbid(cid(0)); // C forbids A(0)
        let dnf = c_constraint.to_dnf();
        assert!(!dnf.satisfied_by(&bits(&[0, 1, 2])));
    }

    use crate::{component::Component, world::World};

    #[derive(Component, Default)]
    struct Health;

    #[derive(Component, Default)]
    struct Mana;

    // Player requires Health via constraint
    #[derive(Component)]
    #[constraint(require(Health))]
    struct Player;

    // Ally forbids Enemy
    #[derive(Component, Default)]
    #[constraint(forbid(Enemy))]
    struct Ally;

    #[derive(Component, Default)]
    struct Enemy;

    // Caster requires either Mana or Scroll
    #[derive(Component, Default)]
    struct Scroll;

    #[derive(Component)]
    #[constraint(or(require(Mana), require(Scroll)))]
    struct Caster;

    // Warrior can only coexist with Health and Armor, nothing else
    #[derive(Component, Default)]
    struct Armor;

    #[derive(Component)]
    #[constraint(only(Health, Armor))]
    struct Warrior;

    // Knight has both only + expr constraints
    #[derive(Component)]
    #[constraint(require(Health))]
    #[constraint(only(Health, Armor))]
    struct Knight;

    #[test]
    fn constraint_only_satisfied() {
        let mut world = World::new();
        // Warrior + Health + Armor -> all in only set -> ok
        let e = world.spawn((Warrior, Health, Armor)).id();
        assert!(world.entity(e).contains::<Warrior>());
        assert!(world.entity(e).contains::<Health>());
        assert!(world.entity(e).contains::<Armor>());
    }

    #[test]
    fn constraint_only_subset_satisfied() {
        let mut world = World::new();
        // Warrior + Health -> only has {Health, Armor}, subset is fine
        let e = world.spawn((Warrior, Health)).id();
        assert!(world.entity(e).contains::<Warrior>());
    }

    #[test]
    fn constraint_only_violated() {
        let mut world = World::new();
        // Warrior + Health + Enemy -> Enemy not in only set -> rejected
        let e = world.spawn((Warrior, Health, Enemy)).id();
        assert!(!world.entity(e).contains::<Warrior>());
    }

    #[test]
    fn constraint_only_with_insert_violated() {
        let mut world = World::new();
        // Warrior + Health -> ok
        let e = world.spawn((Warrior, Health)).id();
        assert!(world.entity(e).contains::<Warrior>());
        // Insert Enemy -> would violate only -> rejected, entity stays as {Warrior, Health}
        world.entity_mut(e).insert(Enemy);
        assert!(world.entity(e).contains::<Warrior>());
        assert!(!world.entity(e).contains::<Enemy>());
    }

    #[test]
    fn constraint_only_and_expr_satisfied() {
        let mut world = World::new();
        // Knight requires Health + only allows {Health, Armor}
        let e = world.spawn((Knight, Health)).id();
        assert!(world.entity(e).contains::<Knight>());
        assert!(world.entity(e).contains::<Health>());
    }

    #[test]
    fn constraint_only_and_expr_violated_missing() {
        let mut world = World::new();
        // Knight requires Health but not provided -> expr constraint fails
        let e = world.spawn(Knight).id();
        assert!(!world.entity(e).contains::<Knight>());
    }

    #[test]
    fn constraint_only_and_expr_violated_disallowed() {
        let mut world = World::new();
        // Knight + Health + Enemy -> Health satisfies require, but Enemy violates only
        let e = world.spawn((Knight, Health, Enemy)).id();
        assert!(!world.entity(e).contains::<Knight>());
    }

    #[test]
    fn constraint_require_satisfied() {
        let mut world = World::new();
        // Player + Health satisfies require(Health)
        let e = world.spawn((Player, Health)).id();
        assert!(world.entity(e).contains::<Player>());
        assert!(world.entity(e).contains::<Health>());
    }

    #[test]
    fn constraint_require_violated() {
        let mut world = World::new();
        // Player without Health -> constraint violated -> entity stays in empty archetype
        let e = world.spawn(Player).id();
        // RESTRICT: the insert should be rejected
        assert!(!world.entity(e).contains::<Player>());
    }

    #[test]
    fn constraint_forbid_violated() {
        let mut world = World::new();
        // Ally + Enemy -> forbid violated
        let e = world.spawn((Ally, Enemy)).id();
        assert!(!world.entity(e).contains::<Ally>());
        assert!(!world.entity(e).contains::<Enemy>());
    }

    #[test]
    fn constraint_or_branch() {
        let mut world = World::new();
        // Caster + Mana -> or(require(Mana), require(Scroll)) satisfied via first branch
        let e = world.spawn((Caster, Mana)).id();
        assert!(world.entity(e).contains::<Caster>());
    }

    #[test]
    fn constraint_or_violated() {
        let mut world = World::new();
        // Caster alone -> neither Mana nor Scroll
        let e = world.spawn(Caster).id();
        assert!(!world.entity(e).contains::<Caster>());
    }

    #[test]
    fn ghost_entity_can_recover() {
        let mut world = World::new();
        let e = world.spawn(Player).id();
        assert!(!world.entity(e).contains::<Player>());

        world.entity_mut(e).insert((Player, Health));
        assert!(world.entity(e).contains::<Player>());
        assert!(world.entity(e).contains::<Health>());
    }

    #[test]
    fn ghost_entity_can_despawn() {
        let mut world = World::new();
        let e = world.spawn(Player).id();
        assert!(!world.entity(e).contains::<Player>());

        // despawn should work
        world.despawn(e);
        assert!(world.get_entity(e).is_err());
    }
}
