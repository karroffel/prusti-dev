// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::super::borrows::Borrow;
use encoder::vir::ast::*;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::mem::discriminant;

#[derive(Debug, Clone)]
pub enum Expr {
    /// A local var
    Local(LocalVar, Position),
    /// An enum variant: base, variant index.
    Variant(Box<Expr>, Field, Position),
    /// A field access
    Field(Box<Expr>, Field, Position),
    /// The inverse of a `val_ref` field access
    AddrOf(Box<Expr>, Type, Position),
    LabelledOld(String, Box<Expr>, Position),
    Const(Const, Position),
    /// lhs, rhs, borrow, position
    MagicWand(Box<Expr>, Box<Expr>, Option<Borrow>, Position),
    /// PredicateAccessPredicate: predicate_name, arg, permission amount
    PredicateAccessPredicate(String, Box<Expr>, PermAmount, Position),
    FieldAccessPredicate(Box<Expr>, PermAmount, Position),
    UnaryOp(UnaryOpKind, Box<Expr>, Position),
    BinOp(BinOpKind, Box<Expr>, Box<Expr>, Position),
    /// Unfolding: predicate name, predicate_args, in_expr, permission amount, enum variant
    Unfolding(String, Vec<Expr>, Box<Expr>, PermAmount, MaybeEnumVariantIndex, Position),
    /// Cond: guard, then_expr, else_expr
    Cond(Box<Expr>, Box<Expr>, Box<Expr>, Position),
    /// ForAll: variables, triggers, body
    ForAll(Vec<LocalVar>, Vec<Trigger>, Box<Expr>, Position),
    /// let variable == (expr) in body
    LetExpr(LocalVar, Box<Expr>, Box<Expr>, Position),
    /// FuncApp: function_name, args, formal_args, return_type, Viper position
    FuncApp(String, Vec<Expr>, Vec<LocalVar>, Type, Position),
}

/// A component that can be used to represent a place as a vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceComponent {
    Field(Field, Position),
    Variant(Field, Position),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOpKind {
    Not,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOpKind {
    EqCmp,
    GtCmp,
    GeCmp,
    LtCmp,
    LeCmp,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Implies,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Const {
    Bool(bool),
    Int(i64),
    BigInt(String),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Local(ref v, ref _pos) => write!(f, "{}", v),
            Expr::Variant(ref base, ref variant_index, ref _pos) => {
                write!(f, "{}[{}]", base, variant_index)
            }
            Expr::Field(ref base, ref field, ref _pos) => write!(f, "{}.{}", base, field),
            Expr::AddrOf(ref base, _, ref _pos) => write!(f, "&({})", base),
            Expr::Const(ref value, ref _pos) => write!(f, "{}", value),
            Expr::BinOp(op, ref left, ref right, ref _pos) => {
                write!(f, "({}) {} ({})", left, op, right)
            }
            Expr::UnaryOp(op, ref expr, ref _pos) => write!(f, "{}({})", op, expr),
            Expr::PredicateAccessPredicate(ref pred_name, ref arg, perm, ref _pos) => {
                write!(f, "acc({}({}), {})", pred_name, arg, perm)
            }
            Expr::FieldAccessPredicate(ref expr, perm, ref _pos) => {
                write!(f, "acc({}, {})", expr, perm)
            }
            Expr::LabelledOld(ref label, ref expr, ref _pos) => {
                write!(f, "old[{}]({})", label, expr)
            }
            Expr::MagicWand(ref left, ref right, ref borrow, ref _pos) => {
                write!(f, "({}) {:?} --* ({})", left, borrow, right)
            }
            Expr::Unfolding(ref pred_name, ref args, ref expr, perm, ref variant, ref _pos) => {
                write!(
                    f,
                    "(unfolding acc({}:{:?}({}), {}) in {})",
                    pred_name,
                    variant,
                    args.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    perm,
                    expr
                )
            },
            Expr::Cond(ref guard, ref left, ref right, ref _pos) => {
                write!(f, "({})?({}):({})", guard, left, right)
            }
            Expr::ForAll(ref vars, ref triggers, ref body, ref _pos) => write!(
                f,
                "forall {} {} :: {}",
                vars.iter()
                    .map(|x| format!("{:?}", x))
                    .collect::<Vec<String>>()
                    .join(", "),
                triggers
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                body.to_string()
            ),
            Expr::LetExpr(ref var, ref expr, ref body, ref _pos) => write!(
                f,
                "(let {:?} == ({}) in {})",
                var,
                expr.to_string(),
                body.to_string()
            ),
            Expr::FuncApp(ref name, ref args, ref params, ref typ, ref _pos) => write!(
                f,
                "{}<{},{}>({})",
                name,
                params
                    .iter()
                    .map(|p| p.typ.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                typ.to_string(),
                args.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
        }
    }
}

impl fmt::Display for UnaryOpKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &UnaryOpKind::Not => write!(f, "!"),
            &UnaryOpKind::Minus => write!(f, "-"),
        }
    }
}

impl fmt::Display for BinOpKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &BinOpKind::EqCmp => write!(f, "=="),
            &BinOpKind::GtCmp => write!(f, ">"),
            &BinOpKind::GeCmp => write!(f, ">="),
            &BinOpKind::LtCmp => write!(f, "<"),
            &BinOpKind::LeCmp => write!(f, "<="),
            &BinOpKind::Add => write!(f, "+"),
            &BinOpKind::Sub => write!(f, "-"),
            &BinOpKind::Mul => write!(f, "*"),
            &BinOpKind::Div => write!(f, "\\"),
            &BinOpKind::Mod => write!(f, "%"),
            &BinOpKind::And => write!(f, "&&"),
            &BinOpKind::Or => write!(f, "||"),
            &BinOpKind::Implies => write!(f, "==>"),
        }
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Const::Bool(val) => write!(f, "{}", val),
            &Const::Int(val) => write!(f, "{}", val),
            &Const::BigInt(ref val) => write!(f, "{}", val),
        }
    }
}

impl Expr {
    pub fn pos(&self) -> &Position {
        match self {
            Expr::Local(_, ref p) => p,
            Expr::Variant(_, _, ref p) => p,
            Expr::Field(_, _, ref p) => p,
            Expr::AddrOf(_, _, ref p) => p,
            Expr::Const(_, ref p) => p,
            Expr::LabelledOld(_, _, ref p) => p,
            Expr::MagicWand(_, _, _, ref p) => p,
            Expr::PredicateAccessPredicate(_, _, _, ref p) => p,
            Expr::FieldAccessPredicate(_, _, ref p) => p,
            Expr::UnaryOp(_, _, ref p) => p,
            Expr::BinOp(_, _, _, ref p) => p,
            Expr::Unfolding(_, _, _, _, _, ref p) => p,
            Expr::Cond(_, _, _, ref p) => p,
            Expr::ForAll(_, _, _, ref p) => p,
            Expr::LetExpr(_, _, _, ref p) => p,
            Expr::FuncApp(_, _, _, _, ref p) => p,
        }
    }

    pub fn set_pos(self, pos: Position) -> Self {
        match self {
            Expr::Local(v, _) => Expr::Local(v, pos),
            Expr::Variant(base, variant_index, _) => Expr::Variant(base, variant_index, pos),
            Expr::Field(e, f, _) => Expr::Field(e, f, pos),
            Expr::AddrOf(e, t, _) => Expr::AddrOf(e, t, pos),
            Expr::Const(x, _) => Expr::Const(x, pos),
            Expr::LabelledOld(x, y, _) => Expr::LabelledOld(x, y, pos),
            Expr::MagicWand(x, y, b, _) => Expr::MagicWand(x, y, b, pos),
            Expr::PredicateAccessPredicate(x, y, z, _) => {
                Expr::PredicateAccessPredicate(x, y, z, pos)
            }
            Expr::FieldAccessPredicate(x, y, _) => Expr::FieldAccessPredicate(x, y, pos),
            Expr::UnaryOp(x, y, _) => Expr::UnaryOp(x, y, pos),
            Expr::BinOp(x, y, z, _) => Expr::BinOp(x, y, z, pos),
            Expr::Unfolding(x, y, z, perm, variant, _) => {
                Expr::Unfolding(x, y, z, perm, variant, pos)
            },
            Expr::Cond(x, y, z, _) => Expr::Cond(x, y, z, pos),
            Expr::ForAll(x, y, z, _) => Expr::ForAll(x, y, z, pos),
            Expr::LetExpr(x, y, z, _) => Expr::LetExpr(x, y, z, pos),
            Expr::FuncApp(x, y, z, k, _) => Expr::FuncApp(x, y, z, k, pos),
        }
    }

    // Replace all Position::default() positions with `pos`
    pub fn set_default_pos(self, pos: Position) -> Self {
        struct DefaultPosReplacer {
            new_pos: Position,
        };
        impl ExprFolder for DefaultPosReplacer {
            fn fold(&mut self, e: Expr) -> Expr {
                let expr = default_fold_expr(self, e);
                if expr.pos().is_default() {
                    expr.set_pos(self.new_pos.clone())
                } else {
                    expr
                }
            }
        }
        DefaultPosReplacer { new_pos: pos }.fold(self)
    }

    pub fn predicate_access_predicate<S: ToString>(name: S, place: Expr, perm: PermAmount) -> Self {
        let pos = place.pos().clone();
        Expr::PredicateAccessPredicate(name.to_string(), box place, perm, pos)
    }

    pub fn pred_permission(place: Expr, perm: PermAmount) -> Option<Self> {
        place
            .typed_ref_name()
            .map(|pred_name| Expr::predicate_access_predicate(pred_name, place, perm))
    }

    pub fn acc_permission(place: Expr, perm: PermAmount) -> Self {
        Expr::FieldAccessPredicate(box place, perm, Position::default())
    }

    pub fn labelled_old(label: &str, expr: Expr) -> Self {
        Expr::LabelledOld(label.to_string(), box expr, Position::default())
    }

    pub fn not(expr: Expr) -> Self {
        Expr::UnaryOp(UnaryOpKind::Not, box expr, Position::default())
    }

    pub fn minus(expr: Expr) -> Self {
        Expr::UnaryOp(UnaryOpKind::Minus, box expr, Position::default())
    }

    pub fn gt_cmp(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::GtCmp, box left, box right, Position::default())
    }

    pub fn ge_cmp(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::GeCmp, box left, box right, Position::default())
    }

    pub fn lt_cmp(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::LtCmp, box left, box right, Position::default())
    }

    pub fn le_cmp(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::LeCmp, box left, box right, Position::default())
    }

    pub fn eq_cmp(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::EqCmp, box left, box right, Position::default())
    }

    pub fn ne_cmp(left: Expr, right: Expr) -> Self {
        Expr::not(Expr::eq_cmp(left, right))
    }

    pub fn add(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Add, box left, box right, Position::default())
    }

    pub fn sub(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Sub, box left, box right, Position::default())
    }

    pub fn mul(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Mul, box left, box right, Position::default())
    }

    pub fn div(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Div, box left, box right, Position::default())
    }

    pub fn modulo(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Mod, box left, box right, Position::default())
    }

    /// Encode Rust reminder. This is *not* Viper modulo.
    pub fn rem(left: Expr, right: Expr) -> Self {
        let abs_right = Expr::ite(
            Expr::ge_cmp(right.clone(), 0.into()),
            right.clone(),
            Expr::minus(right.clone()),
        );
        Expr::ite(
            Expr::or(
                Expr::ge_cmp(left.clone(), 0.into()),
                Expr::eq_cmp(Expr::modulo(left.clone(), right.clone()), 0.into()),
            ),
            // positive value or left % right == 0
            Expr::modulo(left.clone(), right.clone()),
            // negative value
            Expr::sub(Expr::modulo(left, right), abs_right),
        )
    }

    pub fn and(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::And, box left, box right, Position::default())
    }

    pub fn or(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Or, box left, box right, Position::default())
    }

    pub fn xor(left: Expr, right: Expr) -> Self {
        Expr::not(Expr::eq_cmp(left, right))
    }

    pub fn implies(left: Expr, right: Expr) -> Self {
        Expr::BinOp(BinOpKind::Implies, box left, box right, Position::default())
    }

    pub fn forall(vars: Vec<LocalVar>, triggers: Vec<Trigger>, body: Expr) -> Self {
        Expr::ForAll(vars, triggers, box body, Position::default())
    }

    pub fn ite(guard: Expr, left: Expr, right: Expr) -> Self {
        Expr::Cond(box guard, box left, box right, Position::default())
    }

    pub fn unfolding(
        pred_name: String,
        args: Vec<Expr>,
        expr: Expr,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex,
    ) -> Self {
        Expr::Unfolding(pred_name, args, box expr, perm, variant, Position::default())
    }

    /// Create `unfolding T(arg) in body` where `T` is the type of `arg`.
    pub fn wrap_in_unfolding(arg: Expr, body: Expr) -> Expr {
        let type_name = arg.get_type().name();
        let pos = body.pos().clone();
        Expr::Unfolding(type_name, vec![arg], box body, PermAmount::Read, None, pos)
    }

    pub fn func_app(
        name: String,
        args: Vec<Expr>,
        internal_args: Vec<LocalVar>,
        return_type: Type,
        pos: Position,
    ) -> Self {
        Expr::FuncApp(name, args, internal_args, return_type, pos)
    }

    pub fn magic_wand(lhs: Expr, rhs: Expr, borrow: Option<Borrow>) -> Self {
        Expr::MagicWand(box lhs, box rhs, borrow, Position::default())
    }

    pub fn find(&self, sub_target: &Expr) -> bool {
        pub struct ExprFinder<'a> {
            sub_target: &'a Expr,
            found: bool,
        }
        impl<'a> ExprWalker for ExprFinder<'a> {
            fn walk(&mut self, expr: &Expr) {
                if expr == self.sub_target || (expr.is_place() && expr == self.sub_target) {
                    self.found = true;
                } else {
                    default_walk_expr(self, expr)
                }
            }
        }

        let mut finder = ExprFinder {
            sub_target,
            found: false,
        };
        finder.walk(self);
        finder.found
    }

    /// Extract all predicates places mentioned in the expression whose predicates have the given
    /// permission amount.
    pub fn extract_predicate_places(&self, perm_amount: PermAmount) -> Vec<Expr> {
        pub struct PredicateFinder {
            predicates: Vec<Expr>,
            perm_amount: PermAmount,
        }
        impl ExprWalker for PredicateFinder {
            fn walk_predicate_access_predicate(
                &mut self,
                _name: &str,
                arg: &Expr,
                perm_amount: PermAmount,
                _pos: &Position
            ) {
                if perm_amount == self.perm_amount {
                    self.predicates.push(arg.clone());
                }
            }
        }

        let mut finder = PredicateFinder {
            predicates: Vec::new(),
            perm_amount: perm_amount,
        };
        finder.walk(self);
        finder.predicates
    }

    /// Split place into place components.
    pub fn explode_place(&self) -> (Expr, Vec<PlaceComponent>) {
        match self {
            Expr::Variant(ref base, ref variant, ref pos) => {
                let (base_base, mut components) = base.explode_place();
                components.push(PlaceComponent::Variant(variant.clone(), pos.clone()));
                (base_base, components)
            }
            Expr::Field(ref base, ref field, ref pos) => {
                let (base_base, mut components) = base.explode_place();
                components.push(PlaceComponent::Field(field.clone(), pos.clone()));
                (base_base, components)
            }
            _ => (self.clone(), vec![]),
        }
    }

    /// Reconstruct place from the place components.
    pub fn reconstruct_place(self, components: Vec<PlaceComponent>) -> Expr {
        components
            .into_iter()
            .fold(self, |acc, component| match component {
                PlaceComponent::Variant(variant, pos) => Expr::Variant(box acc, variant, pos),
                PlaceComponent::Field(field, pos) => Expr::Field(box acc, field, pos),
            })
    }

    // Methods from the old `Place` structure

    pub fn local(local: LocalVar) -> Self {
        Expr::Local(local, Position::default())
    }

    pub fn variant(self, index: &str) -> Self {
        assert!(self.is_place());
        let field_name = format!("enum_{}", index);
        let typ = self.get_type();
        let variant = Field::new(field_name, typ.clone().variant(index));
        Expr::Variant(box self, variant, Position::default())
    }

    pub fn field(self, field: Field) -> Self {
        Expr::Field(box self, field, Position::default())
    }

    pub fn addr_of(self) -> Self {
        let type_name = self.get_type().name();
        Expr::AddrOf(box self, Type::TypedRef(type_name), Position::default())
    }

    pub fn is_only_permissions(&self) -> bool {
        match self {
            Expr::PredicateAccessPredicate(..) |
            Expr::FieldAccessPredicate(..) => true,
            Expr::BinOp(BinOpKind::And, box lhs, box rhs, _) => {
                lhs.is_only_permissions() && rhs.is_only_permissions()
            }
            _ => false,
        }
    }

    pub fn is_place(&self) -> bool {
        match self {
            &Expr::Local(_, _) => true,
            &Expr::Variant(ref base, _, _)
            | &Expr::Field(ref base, _, _)
            | &Expr::AddrOf(ref base, _, _)
            | &Expr::LabelledOld(_, ref base, _)
            | &Expr::Unfolding(_, _, ref base, _, _, _) => base.is_place(),
            _ => false,
        }
    }

    pub fn is_variant(&self) -> bool {
        match self {
            Expr::Variant(..) => true,
            _ => false,
        }
    }

    /// How many parts this place has? Used for ordering places.
    pub fn place_depth(&self) -> u32 {
        match self {
            &Expr::Local(_, _) => 1,
            &Expr::Variant(ref base, _, _)
            | &Expr::Field(ref base, _, _)
            | &Expr::AddrOf(ref base, _, _)
            | &Expr::LabelledOld(_, ref base, _)
            | &Expr::Unfolding(_, _, ref base, _, _, _) => base.place_depth() + 1,
            x => unreachable!("{:?}", x),
        }
    }

    pub fn is_simple_place(&self) -> bool {
        match self {
            &Expr::Local(_, _) => true,
            &Expr::Variant(ref base, _, _) | &Expr::Field(ref base, _, _) => base.is_simple_place(),
            _ => false,
        }
    }

    /// Only defined for places
    pub fn get_parent_ref(&self) -> Option<&Expr> {
        debug_assert!(self.is_place());
        match self {
            &Expr::Local(_, _) => None,
            &Expr::Variant(box ref base, _, _)
            | &Expr::Field(box ref base, _, _)
            | &Expr::AddrOf(box ref base, _, _) => Some(base),
            &Expr::LabelledOld(_, _, _) => None,
            &Expr::Unfolding(_, _, _, _, _, _) => None,
            ref x => unreachable!("{}", x),
        }
    }

    /// Only defined for places
    pub fn get_parent(&self) -> Option<Expr> {
        self.get_parent_ref().cloned()
    }

    /// Is this place a MIR reference?
    pub fn is_mir_reference(&self) -> bool {
        debug_assert!(self.is_place());
        if let Expr::Field(box Expr::Local(LocalVar { typ, .. }, _), _, _) = self {
            if let Type::TypedRef(ref name) = typ {
                // FIXME: We should not rely on string names for detecting types.
                return name.starts_with("ref$");
            }
        }
        false
    }

    /// If self is a MIR reference, dereference it.
    pub fn try_deref(&self) -> Option<Self> {
        if let Type::TypedRef(ref predicate_name) = self.get_type() {
            // FIXME: We should not rely on string names for type conversions.
            if predicate_name.starts_with("ref$") {
                let field_predicate_name = predicate_name[4..predicate_name.len()].to_string();
                let field = Field::new("val_ref", Type::TypedRef(field_predicate_name));
                let field_place = Expr::from(self.clone()).field(field);
                return Some(field_place);
            }
        }
        None
    }

    pub fn is_local(&self) -> bool {
        match self {
            &Expr::Local(..) => true,
            _ => false,
        }
    }

    pub fn is_addr_of(&self) -> bool {
        match self {
            &Expr::AddrOf(..) => true,
            _ => false,
        }
    }

    /// Puts an `old[label](..)` around the expression
    pub fn old<S: fmt::Display + ToString>(self, label: S) -> Self {
        match self {
            Expr::Local(..) => {
                /*
                debug!(
                    "Trying to put an old expression 'old[{}](..)' around {}, which is a local variable",
                    label,
                    self
                );
                */
                self
            }
            Expr::LabelledOld(..) => {
                /*
                debug!(
                    "Trying to put an old expression 'old[{}](..)' around {}, which already has a label",
                    label,
                    self
                );
                */
                self
            }
            _ => Expr::LabelledOld(label.to_string(), box self, Position::default()),
        }
    }

    pub fn is_old(&self) -> bool {
        self.get_label().is_some()
    }

    pub fn is_curr(&self) -> bool {
        !self.is_old()
    }

    pub fn get_place(&self) -> Option<&Expr> {
        match self {
            Expr::PredicateAccessPredicate(_, ref arg, _, _) => Some(arg),
            Expr::FieldAccessPredicate(box ref arg, _, _) => Some(arg),
            _ => None,
        }
    }

    pub fn get_perm_amount(&self) -> PermAmount {
        match self {
            Expr::PredicateAccessPredicate(_, _, perm_amount, _) => *perm_amount,
            Expr::FieldAccessPredicate(_, perm_amount, _) => *perm_amount,
            x => unreachable!("{}", x),
        }
    }

    pub fn is_pure(&self) -> bool {
        struct PurityFinder {
            non_pure: bool,
        }
        impl ExprWalker for PurityFinder {
            fn walk_predicate_access_predicate(
                &mut self,
                _name: &str,
                _arg: &Expr,
                _perm_amount: PermAmount,
                _pos: &Position
            ) {
                self.non_pure = true;
            }
            fn walk_field_access_predicate(
                &mut self,
                _receiver: &Expr,
                _perm_amount: PermAmount,
                _pos: &Position
            ) {
                self.non_pure = true;
            }
        }
        let mut walker = PurityFinder { non_pure: false };
        walker.walk(self);
        !walker.non_pure
    }

    /// Only defined for places
    pub fn get_base(&self) -> LocalVar {
        debug_assert!(self.is_place());
        match self {
            &Expr::Local(ref var, _) => var.clone(),
            &Expr::LabelledOld(_, ref base, _) |
            &Expr::Unfolding(_, _, ref base, _, _, _) => {
                base.get_base()
            }
            _ => self.get_parent().unwrap().get_base(),
        }
    }

    pub fn get_label(&self) -> Option<&String> {
        match self {
            &Expr::LabelledOld(ref label, _, _) => Some(label),
            _ => None,
        }
    }

    /* Moved to the Eq impl
    /// Place equality after type elision
    pub fn weak_eq(&self, other: &Expr) -> bool {
        debug_assert!(self.is_place());
        debug_assert!(other.is_place());
        match (self, other) {
            (
                Expr::Local(ref self_var),
                Expr::Local(ref other_var)
            ) => self_var.weak_eq(other_var),
            (
                Expr::Field(box ref self_base, ref self_field),
                Expr::Field(box ref other_base, ref other_field)
            ) => self_field.weak_eq(other_field) && self_base.weak_eq(other_base),
            (
                Expr::AddrOf(box ref self_base, ref self_typ),
                Expr::AddrOf(box ref other_base, ref other_typ)
            ) => self_typ.weak_eq(other_typ) && self_base.weak_eq(other_base),
            (
                Expr::LabelledOld(ref self_label, box ref self_base),
                Expr::LabelledOld(ref other_label, box ref other_base)
            ) => self_label == other_label && self_base.weak_eq(other_base),
            (
                Expr::Unfolding(ref self_name, ref self_args, box ref self_base, self_frac),
                Expr::Unfolding(ref other_name, ref other_args, box ref other_base, other_frac)
            ) => self_name == other_name && self_frac == other_frac &&
                self_args[0].weak_eq(&other_args[0]) && self_base.weak_eq(other_base),
            _ => false
        }
    }
    */

    pub fn has_proper_prefix(&self, other: &Expr) -> bool {
        debug_assert!(self.is_place(), "self={} other={}", self, other);
        debug_assert!(other.is_place(), "self={} other={}", self, other);
        self != other && self.has_prefix(other)
    }

    pub fn has_prefix(&self, other: &Expr) -> bool {
        debug_assert!(self.is_place());
        debug_assert!(other.is_place());
        if self == other {
            true
        } else {
            match self.get_parent() {
                Some(parent) => parent.has_prefix(other),
                None => false,
            }
        }
    }

    pub fn all_proper_prefixes(&self) -> Vec<Expr> {
        debug_assert!(self.is_place());
        match self.get_parent() {
            Some(parent) => parent.all_prefixes(),
            None => vec![],
        }
    }

    // Returns all prefixes, from the shortest to the longest
    pub fn all_prefixes(&self) -> Vec<Expr> {
        debug_assert!(self.is_place());
        let mut res = self.all_proper_prefixes();
        res.push(self.clone());
        res
    }

    pub fn get_type(&self) -> &Type {
        debug_assert!(self.is_place());
        match self {
            &Expr::Local(LocalVar { ref typ, .. }, _)
            | &Expr::Variant(_, Field { ref typ, .. }, _)
            | &Expr::Field(_, Field { ref typ, .. }, _)
            | &Expr::AddrOf(_, ref typ, _) => {
                &typ
            },
            &Expr::LabelledOld(_, box ref base, _)
            | &Expr::Unfolding(_, _, box ref base, _, _, _) => {
                base.get_type()
            }
            _ => panic!(),
        }
    }

    pub fn typed_ref_name(&self) -> Option<String> {
        match self.get_type() {
            &Type::TypedRef(ref name) => Some(name.clone()),
            _ => None,
        }
    }

    pub fn map_labels<F>(self, f: F) -> Self
    where
        F: Fn(String) -> Option<String>,
    {
        struct OldLabelReplacer<T: Fn(String) -> Option<String>> {
            f: T,
        };
        impl<T: Fn(String) -> Option<String>> ExprFolder for OldLabelReplacer<T> {
            fn fold_labelled_old(&mut self, label: String, base: Box<Expr>, pos: Position) -> Expr {
                match (self.f)(label) {
                    Some(new_label) => base.old(new_label).set_pos(pos),
                    None => *base,
                }
            }
        }
        OldLabelReplacer { f }.fold(self)
    }

    pub fn replace_place(self, target: &Expr, replacement: &Expr) -> Self {
        debug_assert!(target.is_place());
        //assert_eq!(target.get_type(), replacement.get_type());
        if replacement.is_place() {
            assert!(
                target.get_type() == replacement.get_type(),
                "Cannot substitute '{}' with '{}', because they have incompatible types '{}' and '{}'",
                target,
                replacement,
                target.get_type(),
                replacement.get_type()
            );
        }
        struct PlaceReplacer<'a> {
            target: &'a Expr,
            replacement: &'a Expr,
            // FIXME: the following fields serve a grotesque hack.
            //  Purpose:  Generics. When a less-generic function-under-test desugars specs from
            //            a more-generic function, the vir::Expr contains Local's with __TYPARAM__s,
            //            but Field's with the function-under-test's concrete types. The purpose is
            //            the to "fix" the (Viper) predicates of the fields, i.e. replace those
            //            typarams with local (more) concrete types.
            //            THIS IS FRAGILE!
            typaram_substs: Option<typaram::Substs>,
            subst: bool,
        };
        impl<'a> ExprFolder for PlaceReplacer<'a> {
            fn fold(&mut self, e: Expr) -> Expr {
                if e.is_place() && &e == self.target {
                    self.subst = true;
                    self.replacement.clone()
                } else {
                    match default_fold_expr(self, e) {
                        Expr::Field(expr, mut field, pos) => {
                            if let Some(ts) = &self.typaram_substs {
                                if self.subst && field.typ.is_ref() {
                                    let inner1 = field.typ.name();
                                    let inner2 = ts.apply(&inner1);
                                    debug!("replacing:\n{}\n{}\n========", &inner1, &inner2);
                                    field = Field::new(field.name, Type::TypedRef(inner2));
                                }
                            }
                            Expr::Field(expr, field, pos)
                        }
                        x => {
                            self.subst = false;
                            x
                        }
                    }
                }
            }

            fn fold_forall(
                &mut self,
                vars: Vec<LocalVar>,
                triggers: Vec<Trigger>,
                body: Box<Expr>,
                pos: Position,
            ) -> Expr {
                if vars.contains(&self.target.get_base()) {
                    // Do nothing
                    Expr::ForAll(vars, triggers, body, pos)
                } else {
                    Expr::ForAll(
                        vars,
                        triggers
                            .into_iter()
                            .map(|x| x.replace_place(self.target, self.replacement))
                            .collect(),
                        self.fold_boxed(body),
                        pos,
                    )
                }
            }
        }
        let typaram_substs = match (&target, &replacement) {
            (Expr::Local(tv, _), Expr::Local(rv, _)) => {
                if tv.typ.is_ref() && rv.typ.is_ref() {
                    debug!(
                        "learning:\n{}\n{}\n=======",
                        &target.local_type(),
                        replacement.local_type()
                    );
                    Some(typaram::Substs::learn(
                        &target.local_type(),
                        &replacement.local_type(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };
        PlaceReplacer {
            target,
            replacement,
            typaram_substs,
            subst: false,
        }
        .fold(self)
    }

    /// Replaces expressions like `old[l5](old[l5](_9.val_ref).foo.bar)`
    /// into `old[l5](_9.val_ref.foo.bar)`
    pub fn remove_redundant_old(self) -> Self {
        struct RedundantOldRemover {
            current_label: Option<String>,
        };
        impl ExprFolder for RedundantOldRemover {
            fn fold_labelled_old(&mut self, label: String, base: Box<Expr>, pos: Position) -> Expr {
                let old_current_label = mem::replace(&mut self.current_label, Some(label.clone()));
                let new_base = default_fold_expr(self, *base);
                let new_expr = if Some(label.clone()) == old_current_label {
                    new_base
                } else {
                    new_base.old(label).set_pos(pos)
                };
                self.current_label = old_current_label;
                new_expr
            }
        }
        RedundantOldRemover {
            current_label: None,
        }
        .fold(self)
    }

    /// Leaves a conjunction of `acc(..)` expressions
    pub fn filter_perm_conjunction(self) -> Self {
        struct PermConjunctionFilter();
        impl ExprFolder for PermConjunctionFilter {
            fn fold(&mut self, e: Expr) -> Expr {
                match e {
                    f @ Expr::PredicateAccessPredicate(..) => f,
                    f @ Expr::FieldAccessPredicate(..) => f,
                    Expr::BinOp(BinOpKind::And, y, z, p) => {
                        self.fold_bin_op(BinOpKind::And, y, z, p)
                    }

                    Expr::BinOp(..)
                    | Expr::MagicWand(..)
                    | Expr::Unfolding(..)
                    | Expr::Cond(..)
                    | Expr::UnaryOp(..)
                    | Expr::Const(..)
                    | Expr::Local(..)
                    | Expr::Variant(..)
                    | Expr::Field(..)
                    | Expr::AddrOf(..)
                    | Expr::LabelledOld(..)
                    | Expr::ForAll(..)
                    | Expr::LetExpr(..)
                    | Expr::FuncApp(..) => true.into(),
                }
            }
        }
        PermConjunctionFilter().fold(self)
    }

    /// Apply the closure to all places in the expression.
    pub fn fold_places<F>(self, f: F) -> Expr
    where
        F: Fn(Expr) -> Expr,
    {
        struct PlaceFolder<F>
        where
            F: Fn(Expr) -> Expr,
        {
            f: F,
        };
        impl<F> ExprFolder for PlaceFolder<F>
        where
            F: Fn(Expr) -> Expr,
        {
            fn fold(&mut self, e: Expr) -> Expr {
                if e.is_place() {
                    (self.f)(e)
                } else {
                    default_fold_expr(self, e)
                }
            }
            // TODO: Handle triggers?
        }
        PlaceFolder { f }.fold(self)
    }

    /// Apply the closure to all expressions.
    pub fn fold_expr<F>(self, f: F) -> Expr
    where
        F: Fn(Expr) -> Expr,
    {
        struct ExprFolderImpl<F>
        where
            F: Fn(Expr) -> Expr,
        {
            f: F,
        };
        impl<F> ExprFolder for ExprFolderImpl<F>
        where
            F: Fn(Expr) -> Expr,
        {
            fn fold(&mut self, e: Expr) -> Expr {
                let new_expr = default_fold_expr(self, e);
                (self.f)(new_expr)
            }
        }
        ExprFolderImpl { f }.fold(self)
    }

    pub fn local_type(&self) -> String {
        match &self {
            Expr::Local(localvar, _) => match &localvar.typ {
                Type::TypedRef(str) => str.clone(),
                _ => panic!("expected Type::TypedRef"),
            },
            _ => panic!("expected Expr::Local"),
        }
    }

    /// Compute the permissions that are needed for this expression to
    /// be successfully evaluated. This is method is used for `fold` and
    /// `exhale` statements inside `package` statements because Silicon
    /// fails to compute which permissions it should take into the magic
    /// wand.
    pub fn compute_footprint(&self, perm_amount: PermAmount) -> Vec<Expr> {
        struct Collector {
            perm_amount: PermAmount,
            perms: Vec<Expr>,
        }
        impl ExprWalker for Collector {
            fn walk_variant(&mut self, e: &Expr, v: &Field, p: &Position) {
                self.walk(e);
                let expr = Expr::Variant(box e.clone(), v.clone(), p.clone());
                let perm = Expr::acc_permission(expr, self.perm_amount);
                self.perms.push(perm);
            }
            fn walk_field(&mut self, e: &Expr, f: &Field, p: &Position) {
                self.walk(e);
                let expr = Expr::Field(box e.clone(), f.clone(), p.clone());
                let perm = Expr::acc_permission(expr, self.perm_amount);
                self.perms.push(perm);
            }
            fn walk_labelled_old(&mut self, _label: &str, _expr: &Expr, _pos: &Position) {
                // Stop recursion.
            }
        }
        let mut collector = Collector {
            perm_amount: perm_amount,
            perms: Vec::new(),
        };
        collector.walk(self);
        collector.perms
    }

    /// FIXME: A hack. Replaces all generic types with their instantiations by using string
    /// substitution.
    pub fn patch_types(self, substs: &HashMap<String, String>) -> Self {
        struct TypePatcher<'a> {
            substs: &'a HashMap<String, String>,
        }
        impl<'a> ExprFolder for TypePatcher<'a> {
            fn fold_predicate_access_predicate(
                &mut self,
                mut predicate_name: String,
                arg: Box<Expr>,
                perm_amount: PermAmount,
                pos: Position,
            ) -> Expr {
                for (typ, subst) in self.substs {
                    predicate_name = predicate_name.replace(typ, subst);
                }
                Expr::PredicateAccessPredicate(
                    predicate_name,
                    self.fold_boxed(arg),
                    perm_amount,
                    pos,
                )
            }
            fn fold_local(&mut self, mut var: LocalVar, pos: Position) -> Expr {
                var.typ = var.typ.patch(self.substs);
                Expr::Local(var, pos)
            }
            fn fold_func_app(
                &mut self,
                name: String,
                args: Vec<Expr>,
                formal_args: Vec<LocalVar>,
                return_type: Type,
                pos: Position,
            ) -> Expr {
                let formal_args = formal_args
                    .into_iter()
                    .map(|mut var| {
                        var.typ = var.typ.patch(self.substs);
                        var
                    })
                    .collect();
                // FIXME: We do not patch the return_type because pure functions cannot return
                // generic values.
                Expr::FuncApp(
                    name,
                    args.into_iter().map(|e| self.fold(e)).collect(),
                    formal_args,
                    return_type,
                    pos,
                )
            }
        }
        let mut patcher = TypePatcher { substs: substs };
        patcher.fold(self)
    }
}

impl PartialEq for Expr {
    /// Compare ignoring the `position` field
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expr::Local(ref self_var, _), Expr::Local(ref other_var, _)) => self_var == other_var,
            (
                Expr::Variant(box ref self_base, ref self_variant, _),
                Expr::Variant(box ref other_base, ref other_variant, _),
            ) => (self_base, self_variant) == (other_base, other_variant),
            (
                Expr::Field(box ref self_base, ref self_field, _),
                Expr::Field(box ref other_base, ref other_field, _),
            ) => (self_base, self_field) == (other_base, other_field),
            (
                Expr::AddrOf(box ref self_base, ref self_typ, _),
                Expr::AddrOf(box ref other_base, ref other_typ, _),
            ) => (self_base, self_typ) == (other_base, other_typ),
            (
                Expr::LabelledOld(ref self_label, box ref self_base, _),
                Expr::LabelledOld(ref other_label, box ref other_base, _),
            ) => (self_label, self_base) == (other_label, other_base),
            (Expr::Const(ref self_const, _), Expr::Const(ref other_const, _)) => {
                self_const == other_const
            }
            (
                Expr::MagicWand(box ref self_lhs, box ref self_rhs, self_borrow, _),
                Expr::MagicWand(box ref other_lhs, box ref other_rhs, other_borrow, _),
            ) => (self_lhs, self_rhs, self_borrow) == (other_lhs, other_rhs, other_borrow),
            (
                Expr::PredicateAccessPredicate(ref self_name, ref self_arg, self_perm, _),
                Expr::PredicateAccessPredicate(ref other_name, ref other_arg, other_perm, _),
            ) => (self_name, self_arg, self_perm) == (other_name, other_arg, other_perm),
            (
                Expr::FieldAccessPredicate(box ref self_base, self_perm, _),
                Expr::FieldAccessPredicate(box ref other_base, other_perm, _),
            ) => (self_base, self_perm) == (other_base, other_perm),
            (
                Expr::UnaryOp(self_op, box ref self_arg, _),
                Expr::UnaryOp(other_op, box ref other_arg, _),
            ) => (self_op, self_arg) == (other_op, other_arg),
            (
                Expr::BinOp(self_op, box ref self_left, box ref self_right, _),
                Expr::BinOp(other_op, box ref other_left, box ref other_right, _),
            ) => (self_op, self_left, self_right) == (other_op, other_left, other_right),
            (
                Expr::Cond(box ref self_cond, box ref self_then, box ref self_else, _),
                Expr::Cond(box ref other_cond, box ref other_then, box ref other_else, _),
            ) => (self_cond, self_then, self_else) == (other_cond, other_then, other_else),
            (
                Expr::ForAll(ref self_vars, ref self_triggers, box ref self_expr, _),
                Expr::ForAll(ref other_vars, ref other_triggers, box ref other_expr, _),
            ) => (self_vars, self_triggers, self_expr) == (other_vars, other_triggers, other_expr),
            (
                Expr::LetExpr(ref self_var, box ref self_def, box ref self_expr, _),
                Expr::LetExpr(ref other_var, box ref other_def, box ref other_expr, _),
            ) => (self_var, self_def, self_expr) == (other_var, other_def, other_expr),
            (
                Expr::FuncApp(ref self_name, ref self_args, _, _, _),
                Expr::FuncApp(ref other_name, ref other_args, _, _, _),
            ) => (self_name, self_args) == (other_name, other_args),
            (
                Expr::Unfolding(ref self_name, ref self_args, box ref self_base, self_perm, ref self_variant, _),
                Expr::Unfolding(ref other_name, ref other_args, box ref other_base, other_perm, ref other_variant, _),
            ) => {
                (self_name, self_args, self_base, self_perm, self_variant)
                    == (other_name, other_args, other_base, other_perm, other_variant)
            }
            (a, b) => {
                debug_assert_ne!(discriminant(a), discriminant(b));
                false
            }
        }
    }
}

impl Eq for Expr {}

impl Hash for Expr {
    /// Hash ignoring the `position` field
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
        match self {
            Expr::Local(ref var, _) => var.hash(state),
            Expr::Variant(box ref base, variant_index, _) => (base, variant_index).hash(state),
            Expr::Field(box ref base, ref field, _) => (base, field).hash(state),
            Expr::AddrOf(box ref base, ref typ, _) => (base, typ).hash(state),
            Expr::LabelledOld(ref label, box ref base, _) => (label, base).hash(state),
            Expr::Const(ref const_expr, _) => const_expr.hash(state),
            Expr::MagicWand(box ref lhs, box ref rhs, b, _) => (lhs, rhs, b).hash(state),
            Expr::PredicateAccessPredicate(ref name, ref arg, perm, _) => {
                (name, arg, perm).hash(state)
            }
            Expr::FieldAccessPredicate(box ref base, perm, _) => (base, perm).hash(state),
            Expr::UnaryOp(op, box ref arg, _) => (op, arg).hash(state),
            Expr::BinOp(op, box ref left, box ref right, _) => (op, left, right).hash(state),
            Expr::Cond(box ref cond, box ref then_expr, box ref else_expr, _) => {
                (cond, then_expr, else_expr).hash(state)
            }
            Expr::ForAll(ref vars, ref triggers, box ref expr, _) => {
                (vars, triggers, expr).hash(state)
            }
            Expr::LetExpr(ref var, box ref def, box ref expr, _) => (var, def, expr).hash(state),
            Expr::FuncApp(ref name, ref args, _, _, _) => (name, args).hash(state),
            Expr::Unfolding(ref name, ref args, box ref base, perm, ref variant, _) => {
                (name, args, base, perm, variant).hash(state)
            }
        }
    }
}

pub trait ExprFolder: Sized {
    fn fold(&mut self, e: Expr) -> Expr {
        default_fold_expr(self, e)
    }

    fn fold_boxed(&mut self, e: Box<Expr>) -> Box<Expr> {
        box self.fold(*e)
    }

    fn fold_local(&mut self, v: LocalVar, p: Position) -> Expr {
        Expr::Local(v, p)
    }
    fn fold_variant(&mut self, base: Box<Expr>, variant: Field, p: Position) -> Expr {
        Expr::Variant(self.fold_boxed(base), variant, p)
    }
    fn fold_field(&mut self, e: Box<Expr>, f: Field, p: Position) -> Expr {
        Expr::Field(self.fold_boxed(e), f, p)
    }
    fn fold_addr_of(&mut self, e: Box<Expr>, t: Type, p: Position) -> Expr {
        Expr::AddrOf(self.fold_boxed(e), t, p)
    }
    fn fold_const(&mut self, x: Const, p: Position) -> Expr {
        Expr::Const(x, p)
    }
    fn fold_labelled_old(
        &mut self,
        label: String,
        body: Box<Expr>,
        pos: Position
    ) -> Expr {
        Expr::LabelledOld(label, self.fold_boxed(body), pos)
    }
    fn fold_magic_wand(
        &mut self,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        borrow: Option<Borrow>,
        pos: Position,
    ) -> Expr {
        Expr::MagicWand(self.fold_boxed(lhs), self.fold_boxed(rhs), borrow, pos)
    }
    fn fold_predicate_access_predicate(
        &mut self,
        name: String,
        arg: Box<Expr>,
        perm_amount: PermAmount,
        pos: Position,
    ) -> Expr {
        Expr::PredicateAccessPredicate(name, self.fold_boxed(arg), perm_amount, pos)
    }
    fn fold_field_access_predicate(
        &mut self,
        receiver: Box<Expr>,
        perm_amount: PermAmount,
        pos: Position
    ) -> Expr {
        Expr::FieldAccessPredicate(self.fold_boxed(receiver), perm_amount, pos)
    }
    fn fold_unary_op(&mut self, x: UnaryOpKind, y: Box<Expr>, p: Position) -> Expr {
        Expr::UnaryOp(x, self.fold_boxed(y), p)
    }
    fn fold_bin_op(
        &mut self,
        kind: BinOpKind,
        first: Box<Expr>,
        second: Box<Expr>,
        pos: Position
    ) -> Expr {
        Expr::BinOp(kind, self.fold_boxed(first), self.fold_boxed(second), pos)
    }
    fn fold_unfolding(
        &mut self,
        name: String,
        args: Vec<Expr>,
        expr: Box<Expr>,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex,
        pos: Position,
    ) -> Expr {
        Expr::Unfolding(
            name,
            args.into_iter().map(|e| self.fold(e)).collect(),
            self.fold_boxed(expr),
            perm,
            variant,
            pos,
        )
    }
    fn fold_cond(
        &mut self,
        guard: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        pos: Position
    ) -> Expr {
        Expr::Cond(
            self.fold_boxed(guard),
            self.fold_boxed(then_expr),
            self.fold_boxed(else_expr),
            pos,
        )
    }
    fn fold_forall(
        &mut self,
        x: Vec<LocalVar>,
        y: Vec<Trigger>,
        z: Box<Expr>,
        p: Position,
    ) -> Expr {
        Expr::ForAll(x, y, self.fold_boxed(z), p)
    }
    fn fold_let_expr(
        &mut self,
        var: LocalVar,
        expr: Box<Expr>,
        body: Box<Expr>,
        pos: Position
    ) -> Expr {
        Expr::LetExpr(var, self.fold_boxed(expr), self.fold_boxed(body), pos)
    }
    fn fold_func_app(
        &mut self,
        name: String,
        args: Vec<Expr>,
        formal_args: Vec<LocalVar>,
        return_type: Type,
        pos: Position,
    ) -> Expr {
        Expr::FuncApp(
            name,
            args.into_iter().map(|e| self.fold(e)).collect(),
            formal_args,
            return_type,
            pos
        )
    }
}

pub fn default_fold_expr<T: ExprFolder>(this: &mut T, e: Expr) -> Expr {
    match e {
        Expr::Local(v, p) => this.fold_local(v, p),
        Expr::Variant(base, variant, p) => this.fold_variant(base, variant, p),
        Expr::Field(e, f, p) => this.fold_field(e, f, p),
        Expr::AddrOf(e, t, p) => this.fold_addr_of(e, t, p),
        Expr::Const(x, p) => this.fold_const(x, p),
        Expr::LabelledOld(x, y, p) => this.fold_labelled_old(x, y, p),
        Expr::MagicWand(x, y, b, p) => this.fold_magic_wand(x, y, b, p),
        Expr::PredicateAccessPredicate(x, y, z, p) => {
            this.fold_predicate_access_predicate(x, y, z, p)
        }
        Expr::FieldAccessPredicate(x, y, p) => this.fold_field_access_predicate(x, y, p),
        Expr::UnaryOp(x, y, p) => this.fold_unary_op(x, y, p),
        Expr::BinOp(x, y, z, p) => this.fold_bin_op(x, y, z, p),
        Expr::Unfolding(x, y, z, perm, variant, p) => {
            this.fold_unfolding(x, y, z, perm, variant, p)
        },
        Expr::Cond(x, y, z, p) => this.fold_cond(x, y, z, p),
        Expr::ForAll(x, y, z, p) => this.fold_forall(x, y, z, p),
        Expr::LetExpr(x, y, z, p) => this.fold_let_expr(x, y, z, p),
        Expr::FuncApp(x, y, z, k, p) => this.fold_func_app(x, y, z, k, p),
    }
}

pub trait ExprWalker: Sized {
    fn walk(&mut self, expr: &Expr) {
        default_walk_expr(self, expr);
    }

    fn walk_local_var(&mut self, _var: &LocalVar) {}

    fn walk_local(&mut self, var: &LocalVar, _pos: &Position) {
        self.walk_local_var(var);
    }
    fn walk_variant(&mut self, base: &Expr, _variant: &Field, _pos: &Position) {
        self.walk(base);
    }
    fn walk_field(&mut self, receiver: &Expr, _field: &Field, _pos: &Position) {
        self.walk(receiver);
    }
    fn walk_addr_of(&mut self, receiver: &Expr, _typ: &Type, _pos: &Position) {
        self.walk(receiver);
    }
    fn walk_const(&mut self, _const: &Const, _pos: &Position) {}
    fn walk_labelled_old(&mut self, _label: &str, body: &Expr, _pos: &Position) {
        self.walk(body);
    }
    fn walk_magic_wand(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        _borrow: &Option<Borrow>,
        _pos: &Position
    ) {
        self.walk(lhs);
        self.walk(rhs);
    }
    fn walk_predicate_access_predicate(
        &mut self,
        _name: &str,
        arg: &Expr,
        _perm_amount: PermAmount,
        _pos: &Position
    ) {
        self.walk(arg)
    }
    fn walk_field_access_predicate(
        &mut self,
        receiver: &Expr,
        _perm_amount: PermAmount,
        _pos: &Position
    ) {
        self.walk(receiver)
    }
    fn walk_unary_op(&mut self, _op: UnaryOpKind, arg: &Expr, _pos: &Position) {
        self.walk(arg)
    }
    fn walk_bin_op(&mut self, _op: BinOpKind, arg1: &Expr, arg2: &Expr, _pos: &Position) {
        self.walk(arg1);
        self.walk(arg2);
    }
    fn walk_unfolding(
        &mut self,
        _name: &str,
        args: &Vec<Expr>,
        body: &Expr,
        _perm: PermAmount,
        _variant: &MaybeEnumVariantIndex,
        _pos: &Position
    ) {
        for arg in args {
            self.walk(arg);
        }
        self.walk(body);
    }
    fn walk_cond(&mut self, guard: &Expr, then_expr: &Expr, else_expr: &Expr, _pos: &Position) {
        self.walk(guard);
        self.walk(then_expr);
        self.walk(else_expr);
    }
    fn walk_forall(
        &mut self,
        vars: &Vec<LocalVar>,
        _triggers: &Vec<Trigger>,
        body: &Expr,
        _pos: &Position
    ) {
        for var in vars {
            self.walk_local_var(var);
        }
        self.walk(body);
    }
    fn walk_let_expr(&mut self, bound_var: &LocalVar, expr: &Expr, body: &Expr, _pos: &Position) {
        self.walk_local_var(bound_var);
        self.walk(expr);
        self.walk(body);
    }
    fn walk_func_app(
        &mut self,
        _name: &str,
        args: &Vec<Expr>,
        formal_args: &Vec<LocalVar>,
        _return_type: &Type,
        _pos: &Position
    ) {
        for arg in args {
            self.walk(arg)
        }
        for arg in formal_args {
            self.walk_local_var(arg);
        }
    }
}

pub fn default_walk_expr<T: ExprWalker>(this: &mut T, e: &Expr) {
    match *e {
        Expr::Local(ref v, ref p) => this.walk_local(v, p),
        Expr::Variant(ref base, ref variant, ref p) => this.walk_variant(base, variant, p),
        Expr::Field(ref e, ref f, ref p) => this.walk_field(e, f, p),
        Expr::AddrOf(ref e, ref t, ref p) => this.walk_addr_of(e, t, p),
        Expr::Const(ref x, ref p) => this.walk_const(x, p),
        Expr::LabelledOld(ref x, ref y, ref p) => this.walk_labelled_old(x, y, p),
        Expr::MagicWand(ref x, ref y, ref b, ref p) => this.walk_magic_wand(x, y, b, p),
        Expr::PredicateAccessPredicate(ref x, ref y, z, ref p) => {
            this.walk_predicate_access_predicate(x, y, z, p)
        }
        Expr::FieldAccessPredicate(ref x, y, ref p) => this.walk_field_access_predicate(x, y, p),
        Expr::UnaryOp(x, ref y, ref p) => this.walk_unary_op(x, y, p),
        Expr::BinOp(x, ref y, ref z, ref p) => this.walk_bin_op(x, y, z, p),
        Expr::Unfolding(ref x, ref y, ref z, perm, ref variant, ref p) => {
            this.walk_unfolding(x, y, z, perm, variant, p)
        },
        Expr::Cond(ref x, ref y, ref z, ref p) => this.walk_cond(x, y, z, p),
        Expr::ForAll(ref x, ref y, ref z, ref p) => this.walk_forall(x, y, z, p),
        Expr::LetExpr(ref x, ref y, ref z, ref p) => this.walk_let_expr(x, y, z, p),
        Expr::FuncApp(ref x, ref y, ref z, ref k, ref p) => this.walk_func_app(x, y, z, k, p),
    }
}

impl Expr {
    /// Remove read permissions. For example, if the expression is
    /// `acc(x.f, read) && acc(P(x.f), write)`, then after the
    /// transformation it will be: `acc(P(x.f), write)`.
    pub fn remove_read_permissions(self) -> Self {
        struct ReadPermRemover {};
        impl ExprFolder for ReadPermRemover {
            fn fold_predicate_access_predicate(
                &mut self,
                name: String,
                arg: Box<Expr>,
                perm_amount: PermAmount,
                p: Position,
            ) -> Expr {
                assert!(perm_amount.is_valid_for_specs());
                match perm_amount {
                    PermAmount::Write => Expr::PredicateAccessPredicate(name, arg, perm_amount, p),
                    PermAmount::Read => true.into(),
                    _ => unreachable!(),
                }
            }
            fn fold_field_access_predicate(
                &mut self,
                reference: Box<Expr>,
                perm_amount: PermAmount,
                p: Position,
            ) -> Expr {
                assert!(perm_amount.is_valid_for_specs());
                match perm_amount {
                    PermAmount::Write => Expr::FieldAccessPredicate(reference, perm_amount, p),
                    PermAmount::Read => true.into(),
                    _ => unreachable!(),
                }
            }
        }
        let mut remover = ReadPermRemover {};
        remover.fold(self)
    }
}

pub trait ExprIterator {
    /// Conjoin a sequence of expressions into a single expression.
    /// Returns true if the sequence has no elements.
    fn conjoin(&mut self) -> Expr;

    /// Disjoin a sequence of expressions into a single expression.
    /// Returns true if the sequence has no elements.
    fn disjoin(&mut self) -> Expr;
}

impl<T> ExprIterator for T
where
    T: Iterator<Item = Expr>,
{
    fn conjoin(&mut self) -> Expr {
        fn rfold<T>(s: &mut T) -> Expr
        where
            T: Iterator<Item = Expr>,
        {
            if let Some(conjunct) = s.next() {
                Expr::and(conjunct, rfold(s))
            } else {
                true.into()
            }
        }
        rfold(self)
    }

    fn disjoin(&mut self) -> Expr {
        fn rfold<T>(s: &mut T) -> Expr
        where
            T: Iterator<Item = Expr>,
        {
            if let Some(conjunct) = s.next() {
                Expr::or(conjunct, rfold(s))
            } else {
                false.into()
            }
        }
        rfold(self)
    }
}
