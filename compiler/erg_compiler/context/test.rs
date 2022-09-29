//! test module for `Context`
use erg_common::error::Location;
use erg_common::Str;
// use erg_common::error::Location;
use erg_common::{enum_unwrap, set};

use erg_type::constructors::{builtin_poly, func1, mono_q, quant, refinement};
use erg_type::typaram::TyParam;
use erg_type::{Predicate, TyBound, Type};
use Type::*;

use crate::context::instantiate::TyVarContext;
use crate::context::Context;

impl Context {
    pub fn test_refinement_subtyping(&self) -> Result<(), ()> {
        // Nat :> {I: Int | I >= 1} ?
        let lhs = Nat;
        let var = Str::ever("I");
        let rhs = refinement(
            var.clone(),
            Type::Int,
            set! { Predicate::eq(var, TyParam::value(1)) },
        );
        if self.supertype_of(&lhs, &rhs) {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn test_resolve_trait_inner1(&self) -> Result<(), ()> {
        let name = Str::ever("Add");
        let params = vec![TyParam::t(Nat)];
        let maybe_trait = builtin_poly(name.clone(), params);
        let mut min = Type::Obj;
        for pair in self.rec_get_trait_impls(&name) {
            if self.supertype_of(&pair.sup_trait, &maybe_trait) {
                min = self.min(&min, &pair.sub_type).unwrap_or(&min).clone();
            }
        }
        if min == Nat {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn test_instantiation_and_generalization(&self) -> Result<(), ()> {
        let t = mono_q("T");
        let eq = builtin_poly("Eq", vec![TyParam::t(t.clone())]);
        let bound = TyBound::subtype_of(t.clone(), eq);
        let bounds = set! {bound};
        let unbound_t = func1(t.clone(), t);
        let quantified = quant(unbound_t.clone(), bounds.clone());
        println!("quantified      : {quantified}");
        let mut tv_ctx = TyVarContext::new(self.level + 1, bounds, self);
        println!("tv_ctx: {tv_ctx}");
        let inst = self
            .instantiate_t(unbound_t, &mut tv_ctx, Location::Unknown)
            .map_err(|_| ())?;
        println!("inst: {inst}");
        let quantified_again = self.generalize_t(inst);
        println!("quantified_again: {quantified_again}");
        assert_eq!(quantified, quantified_again);
        let unbound_t = *enum_unwrap!(quantified_again, Type::Quantified).unbound_callable;
        // 同じtv_ctxで2回instantiateしないこと
        let inst = self
            .instantiate_t(unbound_t, &mut tv_ctx, Location::Unknown)
            .map_err(|_| ())?; // (?T(<: Eq('T))[2]) -> ?T(<: Eq('T))[2]
        println!("inst: {inst}");
        let quantified_again = self.generalize_t(inst);
        println!("quantified_again: {quantified_again}");
        if quantified_again == quantified {
            // 結果的に同じにはなる
            Ok(())
        } else {
            Err(())
        }
    }
}
