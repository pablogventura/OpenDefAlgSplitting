//! Gap-driven Stone filter (Baker-Pixley), not Thm 3.2 index enumeration.
//!
//! State = current ops. While Sub(ops)\S or SubIso(ops)\Γ is nonempty, synthesize
//! one separator (`f_U,b` or `g_D`) from a concrete witness and append it.

use std::collections::HashSet;

use crate::stone::preserve::{
    current_sub_isos, fails_preserve_iso, not_preserves_sub_sq_current, StoneOp,
};
use crate::stone::report::FilterReport;
use crate::stone::spec::{Elem, PartialIso, StoneSpec};

fn sorted_elems(s: &HashSet<Elem>) -> Vec<Elem> {
    let mut v: Vec<Elem> = s.iter().copied().collect();
    v.sort_unstable();
    v
}

fn same_action(phi: &PartialIso, gamma: &PartialIso) -> bool {
    phi.dom == gamma.dom
        && phi.cod == gamma.cod
        && phi.dom.iter().all(|&x| phi.apply(x) == gamma.apply(x))
}

fn gamma_covers(spec: &StoneSpec, phi: &PartialIso) -> bool {
    spec.sub_isos.iter().any(|g| same_action(phi, g))
}

fn extra_subs(spec: &StoneSpec, ops: &[StoneOp]) -> Vec<HashSet<Elem>> {
    let target: HashSet<Vec<Elem>> = spec
        .closure_sets
        .iter()
        .map(|s| sorted_elems(s))
        .collect();
    crate::stone::preserve::subuniverses_of_ops(spec, ops)
        .into_iter()
        .filter(|u| !target.contains(&sorted_elems(u)))
        .collect()
}

fn extra_isos(spec: &StoneSpec, ops: &[StoneOp]) -> Vec<PartialIso> {
    let target_sets: HashSet<Vec<Elem>> = spec
        .closure_sets
        .iter()
        .map(|s| sorted_elems(s))
        .collect();
    current_sub_isos(spec, ops)
        .into_iter()
        .filter(|phi| {
            target_sets.contains(&sorted_elems(&phi.dom))
                && target_sets.contains(&sorted_elems(&phi.cod))
                && !gamma_covers(spec, phi)
        })
        .collect()
}

fn synthesize_sub_separator(spec: &StoneSpec, u: &HashSet<Elem>) -> Option<StoneOp> {
    let c = spec.algebraic_closure(&sorted_elems(u));
    let mut extras: Vec<Elem> = c.difference(u).copied().collect();
    extras.sort_unstable();
    let b = *extras.first()?;
    Some(StoneOp::FBar {
        bar: sorted_elems(u),
        b,
    })
}

fn synthesize_iso_separator(phi: &PartialIso) -> Option<StoneOp> {
    if phi.dom.len() >= 2 {
        Some(StoneOp::GD {
            domain: sorted_elems(&phi.dom),
        })
    } else {
        None
    }
}

fn op_eq(a: &StoneOp, b: &StoneOp) -> bool {
    a == b
}

fn gap_step(spec: &StoneSpec, ops: &mut Vec<StoneOp>) -> bool {
    let extras_u = extra_subs(spec, ops);
    for u in &extras_u {
        if let Some(op) = synthesize_sub_separator(spec, u) {
            if !ops.iter().any(|o| op_eq(o, &op)) && not_preserves_sub_sq_current(spec, &op, ops)
            {
                ops.push(op);
                return true;
            }
        }
    }

    let extras_h = extra_isos(spec, ops);
    for phi in &extras_h {
        if let Some(op) = synthesize_iso_separator(phi) {
            let useful = not_preserves_sub_sq_current(spec, &op, ops)
                || fails_preserve_iso(spec, &op, phi);
            if useful && !ops.iter().any(|o| op_eq(o, &op)) {
                ops.push(op);
                return true;
            }
        }
    }
    false
}

fn raw_op_count(spec: &StoneSpec) -> usize {
    super::filter::index_j(spec).len() + super::filter::index_i(spec).len()
}

/// Gap-driven FilteringFunctions (preferred algorithm).
pub fn gap_driven_filtering(spec: &StoneSpec) -> FilterReport {
    let mut ops = vec![StoneOp::Discriminator];
    // Lean gapFuel = 2^n + (2^n)^2; here we cap the exponent at 12 for RAM.
    let fuel = (1usize << spec.universe.len().min(12)) + 256;
    for _ in 0..fuel {
        if !gap_step(spec, &mut ops) {
            break;
        }
    }
    FilterReport::from_ops(raw_op_count(spec), ops.len(), ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fin2_example33_has_g() {
        let json = include_str!("../../../qfdef/test/fixtures/stone/fin2_example33.json");
        let spec = StoneSpec::parse_json(json).expect("parse");
        let report = gap_driven_filtering(&spec);
        assert!(report.operations.iter().any(|s| s.starts_with("g_")));
        assert_eq!(report.operations[0], "discriminator");
    }
}
