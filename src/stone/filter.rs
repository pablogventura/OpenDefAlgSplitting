//! Thm 3.2 certificate filter: enumerate index I/J candidates, keep those that
//! fail Alg. 2 against the current ops (`filtering_functions_thm32` at crate root).
//! Preferred path is gap-driven (`gap_filter` / crate `filtering_functions`).

use std::collections::HashSet;

use crate::stone::preserve::{not_preserves_sub_sq_current, StoneOp};
use crate::stone::spec::{Elem, StoneSpec};

pub fn injective_tuples(universe_size: usize, k: usize) -> Vec<Vec<Elem>> {
    if k == 0 {
        return vec![vec![]];
    }
    if k > universe_size {
        return vec![];
    }
    let mut out = Vec::new();
    fn rec(
        universe_size: usize,
        k: usize,
        start: Elem,
        cur: &mut Vec<Elem>,
        out: &mut Vec<Vec<Elem>>,
    ) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for x in start..universe_size {
            cur.push(x);
            rec(universe_size, k, x + 1, cur, out);
            cur.pop();
        }
    }
    rec(universe_size, k, 0, &mut Vec::new(), &mut out);
    out
}

pub fn index_j(spec: &StoneSpec) -> Vec<Vec<Elem>> {
    spec.closure_sets
        .iter()
        .filter(|s| s.len() >= 2)
        .map(|s| {
            let mut v: Vec<Elem> = s.iter().copied().collect();
            v.sort_unstable();
            v
        })
        .collect()
}

pub fn index_i(spec: &StoneSpec) -> Vec<StoneOp> {
    let mut ops = Vec::new();
    let n = spec.universe.len();
    for k in 0..n {
        for bar in injective_tuples(n, k) {
            let bar_set: HashSet<Elem> = bar.iter().copied().collect();
            let cbar = spec.algebraic_closure(&bar);
            let mut cbar_elems: Vec<Elem> = cbar.iter().copied().collect();
            cbar_elems.sort_unstable();
            for b in cbar_elems {
                if !bar_set.contains(&b) {
                    ops.push(StoneOp::FBar {
                        bar: bar.clone(),
                        b,
                    });
                }
            }
        }
    }
    ops
}

/// Incremental FilteringFunctions via Thm 3.2 index enumeration + Alg. 2.
/// Prefer `gap_driven_filtering` for the Baker-Pixley gap algorithm.
pub fn filtering_functions(spec: &StoneSpec) -> super::report::FilterReport {
    let mut ops = vec![StoneOp::Discriminator];
    let mut raw = 0usize;
    let mut filtered = 1usize;

    for domain in index_j(spec) {
        raw += 1;
        let gop = StoneOp::GD {
            domain: domain.clone(),
        };
        if not_preserves_sub_sq_current(spec, &gop, &ops) {
            ops.push(gop);
            filtered += 1;
        }
    }

    for fop in index_i(spec) {
        raw += 1;
        if not_preserves_sub_sq_current(spec, &fop, &ops) {
            ops.push(fop);
            filtered += 1;
        }
    }

    super::report::FilterReport::from_ops(raw, filtered, ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stone::spec::StoneSpec;

    #[test]
    fn fin2_example33_report() {
        let json = include_str!("../../../qfdef/test/fixtures/stone/fin2_example33.json");
        let spec = StoneSpec::parse_json(json).expect("parse");
        let report = filtering_functions(&spec);
        assert!(report.filtered_op_count >= 1);
        assert_eq!(
            report.operations.first().map(String::as_str),
            Some("discriminator")
        );
    }
}
